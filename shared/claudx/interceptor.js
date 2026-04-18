// claudx interceptor — M13 + M13c (경제) + M13d (v1/hive 이식)
// Loaded via NODE_OPTIONS="--require .../interceptor.js"
//
// Hooks in claudx_fetch (순서):
//   0. URL 필터 — api.anthropic.com 완성 요청만. oauth/token/feedback/metrics 제외
//   1. budgetBlock() — 일일/월간 cost cap 초과시 synthetic 429 반환
//   2. cacheGet() — request body hash → TTL 24h 캐시 hit 이면 즉시 반환 (API 호출 생략)
//   3. origFetch
//   4. isRateLimited → rotation (Authorization swap + replay)
//   5. recordCost() — usage 파싱 → cost.jsonl append
//   6. freshenUsageCache() — anthropic-ratelimit-* 헤더에서 remaining 추출 → usage-cache.json 즉시 반영
//   7. preemptSwapCheck() — remaining < 임계치면 다음 요청 위해 exhausted 마크
//   8. cacheSet() — 성공 응답 cache 저장 (stream 제외)
//   9. scrubRateLimitHeaders() — UI 경고 원천 차단
//
// Storage:
//   $CLAUDX_STATE/cost.jsonl       — {ts, sid, acct, model, in, out, cache_r, cache_w, usd, cwd}
//   $CLAUDX_STATE/cache/<hash>.json — {ts, status, body, headers, usage}
//   $CLAUDX_STATE/sticky.json       — {sid: {acct, last_used}} (pool.js 사용)
//   $CLAUDX_STATE/rotations.jsonl   — rotation/budget/cache 이벤트 로그 (redact 된 payload)
//
// Env:
//   CLAUDX_BUDGET_DAILY   $5 기본값
//   CLAUDX_BUDGET_MONTHLY $50 기본값  (hive pre_provider 호환)
//   CLAUDX_PREEMPT_PCT    0.15 (remaining < 15% 면 선제 swap 준비)
//   CLAUDX_CACHE_TTL_SEC  86400
//   CLAUDX_NO_CACHE       1 → 캐시 전면 비활성
//   CLAUDX_NO_BUDGET      1 → budget cap 비활성
//   CLAUDX_NO_REDACT      1 → log redact 비활성
//   CLAUDX_MAX_ROT        4
//   CLAUDX_SCRUB          0 → UI 헤더 스크럽 끄기

'use strict';

const fs = require('fs');
const path = require('path');
const crypto = require('crypto');
const pool = require('./pool.js');

const STATE_DIR = pool.paths.STATE_DIR;
const ROT_LOG = path.join(STATE_DIR, 'rotations.jsonl');
const COST_LOG = path.join(STATE_DIR, 'cost.jsonl');
const CACHE_DIR = path.join(STATE_DIR, 'cache');
try { fs.mkdirSync(STATE_DIR, { recursive: true }); } catch (_) {}
try { fs.mkdirSync(CACHE_DIR, { recursive: true }); } catch (_) {}

const MAX_ROT = parseInt(process.env.CLAUDX_MAX_ROT || '4', 10);
const EXH_SEC = parseInt(process.env.CLAUDX_EXH_SEC || '3600', 10);
const DEBUG = process.env.CLAUDX_DEBUG === '1';
const SCRUB = process.env.CLAUDX_SCRUB !== '0';
const BUDGET_DAILY = parseFloat(process.env.CLAUDX_BUDGET_DAILY || '5');
const BUDGET_MONTHLY = parseFloat(process.env.CLAUDX_BUDGET_MONTHLY || '50');
const NO_BUDGET = process.env.CLAUDX_NO_BUDGET === '1';
const PREEMPT_PCT = parseFloat(process.env.CLAUDX_PREEMPT_PCT || '0.15');
const CACHE_TTL = parseInt(process.env.CLAUDX_CACHE_TTL_SEC || '86400', 10);
const NO_CACHE = process.env.CLAUDX_NO_CACHE === '1';
const NO_REDACT = process.env.CLAUDX_NO_REDACT === '1';

// --- Pricing table (USD per 1M tokens) — 대략. 정확 필요시 조정 ---
const PRICE = {
  'claude-opus-4': { in: 15, out: 75 },
  'claude-opus-4-7': { in: 15, out: 75 },
  'opus': { in: 15, out: 75 },
  'claude-sonnet-4': { in: 3, out: 15 },
  'claude-sonnet-4-6': { in: 3, out: 15 },
  'sonnet': { in: 3, out: 15 },
  'claude-haiku-4-5': { in: 0.8, out: 4 },
  'haiku': { in: 0.8, out: 4 },
  '_default': { in: 3, out: 15 },
};

function pricefor(model) {
  if (!model) return PRICE._default;
  if (PRICE[model]) return PRICE[model];
  for (const k of Object.keys(PRICE)) if (k !== '_default' && model.indexOf(k) >= 0) return PRICE[k];
  return PRICE._default;
}

function usdOfUsage(model, u) {
  const p = pricefor(model);
  const inTok = (u.input_tokens || 0) + (u.cache_read_input_tokens || 0) * 0.1 + (u.cache_creation_input_tokens || 0) * 1.25;
  const outTok = u.output_tokens || 0;
  return inTok * p.in / 1e6 + outTok * p.out / 1e6;
}

// --- Redact (M13d / v1e) ---

const REDACT_PATTERNS = [
  [/Bearer\s+[A-Za-z0-9._-]{20,}/g, 'Bearer ***'],
  [/sk-ant-[A-Za-z0-9_-]{20,}/g, 'sk-ant-***'],
  [/sk-[A-Za-z0-9_-]{30,}/g, 'sk-***'],
  [/"(password|api[_-]?key|secret|token)"\s*:\s*"[^"]+"/gi, '"$1":"***"'],
  [/password\s*=\s*\S+/gi, 'password=***'],
  [/\b\d{3}-\d{2}-\d{4}\b/g, '***-**-****'],
  [/\b010-\d{4}-\d{4}\b/g, '010-****-****'],
];

function redact(s) {
  if (NO_REDACT || !s) return s;
  let out = s;
  for (const [re, rep] of REDACT_PATTERNS) out = out.replace(re, rep);
  return out;
}

function logEvent(evt) {
  const line = JSON.stringify({ ts: new Date().toISOString(), pid: process.pid, ...evt });
  try { fs.appendFileSync(ROT_LOG, line + '\n'); } catch (_) {}
  if (DEBUG) try { process.stderr.write('[claudx] ' + line + '\n'); } catch (_) {}
}

// --- URL classification ---

function urlString(input) {
  if (typeof input === 'string') return input;
  if (input && typeof input.url === 'string') return input.url;
  try { return String(input); } catch (_) { return ''; }
}

function isAnthropicCompletion(u) {
  if (!u.includes('api.anthropic.com')) return false;
  if (u.includes('/api/oauth/')) return false;
  if (u.includes('/oauth/token')) return false;
  if (u.includes('/api/claude_cli_feedback')) return false;
  if (u.includes('/api/claude_code/metrics')) return false;
  return true;
}

// --- Header helpers ---

function getAuth(headers) {
  if (!headers) return null;
  try {
    if (typeof Headers !== 'undefined' && headers instanceof Headers) {
      return headers.get('authorization') || headers.get('Authorization');
    }
  } catch (_) {}
  if (Array.isArray(headers)) {
    for (const e of headers) if (e && String(e[0]).toLowerCase() === 'authorization') return e[1];
    return null;
  }
  if (typeof headers === 'object') return headers.Authorization || headers.authorization || null;
  return null;
}

function setAuth(headers, token) {
  const bearer = 'Bearer ' + token;
  try {
    if (typeof Headers !== 'undefined' && headers instanceof Headers) {
      const c = new Headers(headers);
      c.set('Authorization', bearer); c.delete('authorization');
      return c;
    }
  } catch (_) {}
  if (Array.isArray(headers)) {
    const n = headers.filter(e => !e || String(e[0]).toLowerCase() !== 'authorization');
    n.push(['Authorization', bearer]);
    return n;
  }
  if (!headers || typeof headers !== 'object') return { Authorization: bearer };
  const n = {};
  for (const k of Object.keys(headers)) if (k.toLowerCase() !== 'authorization') n[k] = headers[k];
  n.Authorization = bearer;
  return n;
}

function readHeader(resp, name) {
  try { return resp.headers.get(name); } catch (_) { return null; }
}

// --- Rate-limit detection ---

async function isRateLimited(resp) {
  if (resp.status === 429) return { hit: true, reason: 'status_429' };
  if (resp.status === 401) return { hit: true, reason: 'status_401_bearer' };
  if (resp.status < 400) return { hit: false };
  try {
    const clone = resp.clone();
    const txt = await clone.text();
    if (/rate_limit_error/i.test(txt)) return { hit: true, reason: 'body_rate_limit_error' };
    if (/usage[ _]limit/i.test(txt)) return { hit: true, reason: 'body_usage_limit' };
    if (/exceeded your/i.test(txt)) return { hit: true, reason: 'body_exceeded' };
    if (/invalid[ _]bearer/i.test(txt)) return { hit: true, reason: 'body_invalid_bearer' };
  } catch (_) {}
  return { hit: false };
}

// --- Cost ledger (e1) ---

async function parseUsageFromBody(resp) {
  // stream 응답은 여기서는 포기 (cli 가 --output-format json 이면 단일 JSON)
  try {
    const clone = resp.clone();
    const txt = await clone.text();
    if (!txt) return null;
    const trimmed = txt.trim();
    // stream-json 은 여러 라인. 마지막 "type":"result" 에서 usage 추출
    if (trimmed.startsWith('{') && trimmed.endsWith('}')) {
      const j = JSON.parse(trimmed);
      return j.usage || (j.message && j.message.usage) || null;
    }
    // stream-json multi-line
    const lines = trimmed.split('\n').filter(Boolean);
    for (let i = lines.length - 1; i >= 0; i--) {
      try {
        const j = JSON.parse(lines[i]);
        if (j.usage) return j.usage;
        if (j.message && j.message.usage) return j.message.usage;
      } catch (_) {}
    }
  } catch (_) {}
  return null;
}

function modelFromBody(body) {
  if (!body) return null;
  try {
    const j = typeof body === 'string' ? JSON.parse(body) : body;
    return j.model || null;
  } catch (_) { return null; }
}

async function recordCost(resp, { acct, model, sid, cwd }) {
  const u = await parseUsageFromBody(resp);
  if (!u) return;
  const usd = usdOfUsage(model, u);
  const rec = {
    ts: new Date().toISOString(),
    sid: sid || '',
    acct,
    model: model || '',
    in: u.input_tokens || 0,
    out: u.output_tokens || 0,
    cache_r: u.cache_read_input_tokens || 0,
    cache_w: u.cache_creation_input_tokens || 0,
    usd: Number(usd.toFixed(6)),
    cwd: cwd || process.cwd(),
  };
  try { fs.appendFileSync(COST_LOG, JSON.stringify(rec) + '\n'); } catch (_) {}
}

// --- Budget cap (e4) ---

function sumCostSince(sinceEpochSec) {
  if (!fs.existsSync(COST_LOG)) return 0;
  let total = 0;
  try {
    const data = fs.readFileSync(COST_LOG, 'utf8');
    for (const line of data.split('\n')) {
      if (!line) continue;
      try {
        const j = JSON.parse(line);
        const t = Math.floor(new Date(j.ts).getTime() / 1000);
        if (t >= sinceEpochSec) total += (j.usd || 0);
      } catch (_) {}
    }
  } catch (_) {}
  return total;
}

function budgetBlock() {
  if (NO_BUDGET) return null;
  const now = Math.floor(Date.now() / 1000);
  const dayAgo = now - 86400;
  const monAgo = now - 86400 * 30;
  const daily = sumCostSince(dayAgo);
  const monthly = sumCostSince(monAgo);
  if (daily >= BUDGET_DAILY) return { kind: 'daily', spent: daily, cap: BUDGET_DAILY };
  if (monthly >= BUDGET_MONTHLY) return { kind: 'monthly', spent: monthly, cap: BUDGET_MONTHLY };
  return null;
}

function synthetic429(reason, detail) {
  const body = JSON.stringify({
    type: 'error',
    error: { type: 'rate_limit_error', message: 'claudx budget cap reached: ' + reason + ' ($' + (detail.spent || 0).toFixed(2) + ' / cap $' + detail.cap + ')' },
  });
  return new Response(body, { status: 429, headers: { 'content-type': 'application/json' } });
}

// --- Preemptive swap (e2) ---

function preemptSwapCheck(resp, acct) {
  const rem = parseInt(readHeader(resp, 'anthropic-ratelimit-requests-remaining') || '0', 10);
  const lim = parseInt(readHeader(resp, 'anthropic-ratelimit-requests-limit') || '1', 10);
  const tokRem = parseInt(readHeader(resp, 'anthropic-ratelimit-input-tokens-remaining') || '0', 10);
  const tokLim = parseInt(readHeader(resp, 'anthropic-ratelimit-input-tokens-limit') || '1', 10);
  const rPct = lim > 0 ? rem / lim : 1;
  const tPct = tokLim > 0 ? tokRem / tokLim : 1;
  if (rPct < PREEMPT_PCT || tPct < PREEMPT_PCT) {
    // short-term 마크. 완전 exhausted 아니라 당분간만 피함
    pool.markExhausted(acct, 300); // 5분 pause
    logEvent({ event: 'preempt_swap_prepared', acct, req_rem_pct: rPct, tok_rem_pct: tPct });
  }
}

// --- Usage cache freshen (e7) ---

function freshenUsageCache(resp, acct) {
  try {
    const cache = pool.paths.USAGE_CACHE;
    if (!fs.existsSync(cache)) return;
    const obj = JSON.parse(fs.readFileSync(cache, 'utf8'));
    const reqRem = parseInt(readHeader(resp, 'anthropic-ratelimit-requests-remaining') || '-1', 10);
    const reqLim = parseInt(readHeader(resp, 'anthropic-ratelimit-requests-limit') || '-1', 10);
    const reset = readHeader(resp, 'anthropic-ratelimit-requests-reset');
    if (reqRem < 0 || reqLim <= 0) return;
    const pct = Math.round(((reqLim - reqRem) / reqLim) * 100);
    obj[acct] = obj[acct] || {};
    obj[acct].session_pct = pct;
    if (reset) obj[acct]._retry_at = Math.floor(new Date(reset).getTime() / 1000);
    // atomic write
    const tmp = cache + '.tmp.' + process.pid;
    fs.writeFileSync(tmp, JSON.stringify(obj, null, 2));
    fs.renameSync(tmp, cache);
  } catch (_) {}
}

// --- Cache (e3) ---

function cacheHashFromInit(input, init) {
  try {
    const headers = init && init.headers ? init.headers : (input && input.headers) || {};
    // Authorization 는 해시에서 제외 — 계정 독립 캐시
    let authStripped = {};
    if (typeof Headers !== 'undefined' && headers instanceof Headers) {
      for (const [k, v] of headers.entries()) if (k.toLowerCase() !== 'authorization') authStripped[k] = v;
    } else if (Array.isArray(headers)) {
      for (const [k, v] of headers) if (String(k).toLowerCase() !== 'authorization') authStripped[k] = v;
    } else if (typeof headers === 'object') {
      for (const k of Object.keys(headers)) if (k.toLowerCase() !== 'authorization') authStripped[k] = headers[k];
    }
    const body = (init && init.body) || (input && typeof input.text === 'function' ? null : null);
    const key = JSON.stringify({ url: urlString(input), method: (init && init.method) || 'POST', body: typeof body === 'string' ? body : '', hdr: authStripped });
    return crypto.createHash('sha256').update(key).digest('hex');
  } catch (_) { return null; }
}

async function cacheHashFromRequestObject(req) {
  try {
    const headers = {};
    for (const [k, v] of req.headers.entries()) if (k.toLowerCase() !== 'authorization') headers[k] = v;
    const body = req.body ? await req.clone().text() : '';
    const key = JSON.stringify({ url: req.url, method: req.method, body, hdr: headers });
    return crypto.createHash('sha256').update(key).digest('hex');
  } catch (_) { return null; }
}

function cacheGet(hash) {
  if (NO_CACHE || !hash) return null;
  const p = path.join(CACHE_DIR, hash + '.json');
  if (!fs.existsSync(p)) return null;
  try {
    const rec = JSON.parse(fs.readFileSync(p, 'utf8'));
    const age = (Date.now() / 1000) - (rec.ts || 0);
    if (age > CACHE_TTL) return null;
    return rec;
  } catch (_) { return null; }
}

function cacheSet(hash, resp, bodyText) {
  if (NO_CACHE || !hash || !bodyText) return;
  try {
    const hdrs = {};
    try { for (const [k, v] of resp.headers.entries()) hdrs[k] = v; } catch (_) {}
    const rec = { ts: Math.floor(Date.now() / 1000), status: resp.status, body: bodyText, headers: hdrs };
    const p = path.join(CACHE_DIR, hash + '.json');
    const tmp = p + '.tmp.' + process.pid;
    fs.writeFileSync(tmp, JSON.stringify(rec));
    fs.renameSync(tmp, p);
  } catch (_) {}
}

function respFromCache(rec) {
  const headers = new Headers(rec.headers || {});
  headers.set('x-claudx-cache', 'hit');
  return new Response(rec.body, { status: rec.status || 200, headers });
}

// --- UI hide: scrub rate-limit headers ---

function scrubRateLimitHeaders(resp) {
  if (!SCRUB) return resp;
  let touched = false;
  const headers = new Headers(resp.headers);
  for (const h of Array.from(headers.keys())) {
    const lk = h.toLowerCase();
    if (lk.startsWith('anthropic-ratelimit-') || lk === 'x-ratelimit-remaining' || lk === 'x-ratelimit-limit' || lk === 'x-ratelimit-reset' || lk === 'retry-after') {
      headers.delete(h); touched = true;
    }
  }
  if (!touched) return resp;
  return new Response(resp.body, { status: resp.status, statusText: resp.statusText, headers });
}

// --- Request rebuild for rotation ---

async function rebuildInput(origInput, newHeaders) {
  if (origInput && typeof origInput === 'object' && 'url' in origInput && 'method' in origInput) {
    try {
      const method = (origInput.method || 'GET').toUpperCase();
      const body = method !== 'GET' && method !== 'HEAD' ? await origInput.clone().arrayBuffer() : undefined;
      return new Request(origInput.url, {
        method: origInput.method, headers: newHeaders, body,
        credentials: origInput.credentials, cache: origInput.cache, redirect: origInput.redirect,
        referrer: origInput.referrer, integrity: origInput.integrity, keepalive: origInput.keepalive,
        mode: origInput.mode, signal: origInput.signal,
      });
    } catch (_) { return origInput.url; }
  }
  return origInput;
}

// --- Install ---

const origFetch = globalThis.fetch.bind(globalThis);

globalThis.fetch = async function claudx_fetch(input, init) {
  const u = urlString(input);
  if (!isAnthropicCompletion(u)) return origFetch(input, init);

  // Hook 1: budget cap (synthetic 429 before network)
  const block = budgetBlock();
  if (block) {
    logEvent({ event: 'budget_block', kind: block.kind, spent: block.spent, cap: block.cap, url: u.slice(0, 80) });
    return synthetic429(block.kind, block);
  }

  // model / cwd / sid 추출 — logging 용
  let reqBodyStr = '';
  try {
    if (init && typeof init.body === 'string') reqBodyStr = init.body;
    else if (init && init.body) reqBodyStr = '';
    else if (input && typeof input === 'object' && input.body) {
      try { reqBodyStr = await input.clone().text(); } catch (_) {}
    }
  } catch (_) {}
  const reqBodyModel = modelFromBody(reqBodyStr);

  // Hook 2: cache
  let hash = null;
  try {
    if (input && typeof input === 'object' && 'url' in input && !init) {
      hash = await cacheHashFromRequestObject(input);
    } else {
      hash = cacheHashFromInit(input, init);
    }
  } catch (_) {}
  if (hash) {
    const hit = cacheGet(hash);
    if (hit) {
      logEvent({ event: 'cache_hit', hash: hash.slice(0, 16), url: u.slice(0, 80) });
      return respFromCache(hit);
    }
  }

  // 정상 경로 — rotation 루프
  let attempt = 0;
  const triedNames = [pool.currentAccountName()];
  let curInput = input;
  let curInit = init || {};

  while (true) {
    let resp;
    try {
      resp = await origFetch(curInput, curInit);
    } catch (e) { throw e; }

    const verdict = await isRateLimited(resp);

    if (!verdict.hit) {
      // 성공 응답 후처리
      const curAcct = triedNames[triedNames.length - 1];
      try { await recordCost(resp, { acct: curAcct, model: reqBodyModel, cwd: process.cwd() }); } catch (_) {}
      try { freshenUsageCache(resp, curAcct); } catch (_) {}
      try { preemptSwapCheck(resp, curAcct); } catch (_) {}

      // cache set — body peek 필요. stream 이면 skip
      if (hash && !NO_CACHE) {
        try {
          const clone = resp.clone();
          const txt = await clone.text();
          // 휴리스틱: 한 줄 JSON 또는 chunked text 가 완전히 도착했을 때만 캐시
          if (txt && txt.length > 0 && txt.length < 5_000_000) cacheSet(hash, resp, txt);
        } catch (_) {}
      }

      return scrubRateLimitHeaders(resp);
    }

    // rotation 경로
    if (attempt >= MAX_ROT) {
      logEvent({ event: 'rotation_exhausted', url: u.slice(0, 100), attempt, reason: verdict.reason });
      return resp;
    }
    const curName = triedNames[triedNames.length - 1];
    pool.markExhausted(curName, EXH_SEC);
    const next = pool.pickBest(triedNames);
    if (!next) {
      logEvent({ event: 'no_candidate', attempt, tried: triedNames, reason: verdict.reason });
      return resp;
    }
    logEvent({ event: 'rotating', from: curName, to: next.name, attempt, reason: verdict.reason, url: u.slice(0, 100), status: resp.status });
    const initHeaders = curInit.headers;
    const reqHeaders = !initHeaders && curInput && typeof curInput === 'object' && curInput.headers ? curInput.headers : initHeaders;
    const swapped = setAuth(reqHeaders || {}, next.token);
    curInit = Object.assign({}, curInit, { headers: swapped });
    if (curInput && typeof curInput === 'object' && 'url' in curInput && !initHeaders) {
      curInput = await rebuildInput(curInput, swapped);
    }
    triedNames.push(next.name);
    attempt++;
    try { resp.body && resp.body.cancel && resp.body.cancel(); } catch (_) {}
  }
};

logEvent({ event: 'interceptor_installed', version: 'M13+c+d', config_dir: process.env.CLAUDE_CONFIG_DIR || '' });
