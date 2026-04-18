// shared/claudx/pool.js — account pool logic shared between interceptor.js (in-process
// rotation) and bin/claudx (pre-launch best-pick). Stateless module; reads SSOT files
// on every call (cheap; 12-row JSON).
//
// Inputs (env-overridable paths):
//   CLAUDX_POOL   → accounts.json  (list of {name, config_dir})
//   CLAUDX_USAGE  → usage-cache.json (per-acct {week_all_pct, session_pct, error, _retry_at})
//   CLAUDX_STATE  → state dir (exhausted.json, rotations.jsonl)
//
// Output of pickBest:
//   { name, config_dir, token, score }  (null if none available)

'use strict';

const fs = require('fs');
const path = require('path');

const HOME = process.env.HOME || process.env.USERPROFILE;
const STATE_DIR = process.env.CLAUDX_STATE || path.join(HOME, '.airgenome', 'claudx');
const POOL_CFG =
  process.env.CLAUDX_POOL ||
  path.join(HOME, 'Dev', 'nexus', 'shared', '.runtime', 'accounts', 'accounts.json');
const USAGE_CACHE =
  process.env.CLAUDX_USAGE ||
  path.join(HOME, 'Dev', 'nexus', 'shared', '.runtime', 'accounts', 'usage-cache.json');
const EXHAUSTED = path.join(STATE_DIR, 'exhausted.json');
const STICKY = path.join(STATE_DIR, 'sticky.json');

function mkdirp(p) {
  try { fs.mkdirSync(p, { recursive: true }); } catch (_) {}
}

function readJSON(p, fallback) {
  try { return JSON.parse(fs.readFileSync(p, 'utf8')); } catch (_) { return fallback; }
}

// Optional prefix remap for cross-host execution (e.g., Mac accounts.json stores
// /Users/ghost/... but we may run on Linux where those live under $HOME/mac_home).
// CLAUDX_HOME_MAP="/Users/ghost:/home/aiden/mac_home" (single pair) or
// comma-separated for multiple pairs.
function remapPath(p) {
  const raw = process.env.CLAUDX_HOME_MAP || '';
  if (!raw || !p) return p;
  for (const pair of raw.split(',')) {
    const [from, to] = pair.split(':');
    if (from && to && p.startsWith(from)) return to + p.slice(from.length);
  }
  return p;
}

function writeJSON(p, obj) {
  mkdirp(path.dirname(p));
  const tmp = p + '.tmp.' + process.pid;
  try {
    fs.writeFileSync(tmp, JSON.stringify(obj, null, 2));
    fs.renameSync(tmp, p);
  } catch (_) {}
}

function basenameToAcct(p) {
  return path.basename(p || '').replace(/^\.claude-/, '');
}

function currentAccountName() {
  return basenameToAcct(process.env.CLAUDE_CONFIG_DIR || '');
}

function loadPool(excludeNames, opts) {
  const exclude = new Set(excludeNames || []);
  const acc = readJSON(POOL_CFG, { accounts: [] });
  const usage = readJSON(USAGE_CACHE, {});
  const exhausted = readJSON(EXHAUSTED, {});
  const now = Math.floor(Date.now() / 1000);
  const softSession = (opts && opts.sessionCap) || 95;
  const softWeek = (opts && opts.weekCap) || 100;
  const out = [];
  for (const a of acc.accounts || []) {
    if (exclude.has(a.name)) continue;
    const u = usage[a.name] || {};
    const ex = exhausted[a.name] || {};
    if (ex.until && ex.until > now) continue;
    if ((u.week_all_pct || 0) >= softWeek) continue;
    if ((u.session_pct || 0) >= softSession) continue;
    if (u._retry_at && u._retry_at > now) continue;
    const cred = path.join(remapPath(a.config_dir), '.credentials.json');
    const creds = readJSON(cred, null);
    const oauth = creds && creds.claudeAiOauth;
    if (!oauth || !oauth.accessToken) continue;
    if (oauth.expiresAt && oauth.expiresAt < Date.now()) continue;
    out.push({
      name: a.name,
      config_dir: a.config_dir,
      token: oauth.accessToken,
      week_pct: u.week_all_pct || 0,
      session_pct: u.session_pct || 0,
      score: (u.week_all_pct || 0) * 2 + (u.session_pct || 0),
    });
  }
  out.sort((x, y) => x.score - y.score);
  return out;
}

// sticky: session_id ↔ preferred_acct 매핑 (M13d · hive forge/sticky 이식).
// 같은 session_id 는 rotation 불가피할 때만 바꿈 — Anthropic prompt cache 보호.
function stickyGet(sid) {
  if (!sid) return null;
  const st = readJSON(STICKY, {});
  return st[sid] && st[sid].acct ? st[sid].acct : null;
}

function stickySet(sid, acct) {
  if (!sid || !acct) return;
  const st = readJSON(STICKY, {});
  st[sid] = { acct, last_used: new Date().toISOString() };
  writeJSON(STICKY, st);
}

function stickyClear(sid) {
  const st = readJSON(STICKY, {});
  if (sid) delete st[sid]; else for (const k of Object.keys(st)) delete st[k];
  writeJSON(STICKY, st);
}

function pickBest(excludeNames, opts) {
  const sid = (opts && opts.stickyFor) || null;
  const all = loadPool(excludeNames);
  if (sid) {
    const pref = stickyGet(sid);
    if (pref) {
      const hit = all.find(p => p.name === pref);
      if (hit) return hit; // sticky 유효 — 첫 자리로
    }
    if (all[0]) stickySet(sid, all[0].name); // 첫 rotation 은 sticky 세팅
  }
  return all[0] || null;
}

function markExhausted(name, seconds) {
  const ex = readJSON(EXHAUSTED, {});
  const until = Math.floor(Date.now() / 1000) + (seconds || 3600);
  ex[name] = { until, ts: new Date().toISOString() };
  writeJSON(EXHAUSTED, ex);
}

function clearExhausted(name) {
  const ex = readJSON(EXHAUSTED, {});
  if (name) delete ex[name]; else for (const k of Object.keys(ex)) delete ex[k];
  writeJSON(EXHAUSTED, ex);
}

function statusTable() {
  const acc = readJSON(POOL_CFG, { accounts: [] });
  const usage = readJSON(USAGE_CACHE, {});
  const ex = readJSON(EXHAUSTED, {});
  const now = Math.floor(Date.now() / 1000);
  const rows = [];
  for (const a of acc.accounts || []) {
    const u = usage[a.name] || {};
    const e = ex[a.name] || {};
    let tag = 'ok';
    if ((u.week_all_pct || 0) >= 100) tag = 'week_exhausted';
    else if ((u.session_pct || 0) >= 95) tag = 'session_cap';
    else if (u._retry_at && u._retry_at > now) tag = 'retry_at=' + u._retry_at;
    else if (e.until && e.until > now) tag = 'local_exh_until=' + e.until;
    else if (u.error && u.error !== null) tag = String(u.error);
    rows.push({
      name: a.name,
      week: u.week_all_pct || 0,
      session: u.session_pct || 0,
      tag,
    });
  }
  return rows;
}

module.exports = {
  loadPool,
  pickBest,
  markExhausted,
  clearExhausted,
  stickyGet,
  stickySet,
  stickyClear,
  currentAccountName,
  basenameToAcct,
  statusTable,
  paths: { POOL_CFG, USAGE_CACHE, EXHAUSTED, STATE_DIR, STICKY },
};

// CLI: node pool.js {pick|status|clear [name]}
if (require.main === module) {
  const cmd = process.argv[2] || 'pick';
  if (cmd === 'pick') {
    const best = pickBest();
    if (!best) { process.exit(1); }
    // format: <name>\t<config_dir>\t<week_pct>\t<session_pct>
    process.stdout.write([best.name, best.config_dir, best.week_pct, best.session_pct].join('\t') + '\n');
  } else if (cmd === 'status') {
    const rows = statusTable();
    for (const r of rows) {
      process.stdout.write([r.name, r.week + '%', r.session + '%', r.tag].join('\t') + '\n');
    }
  } else if (cmd === 'clear') {
    clearExhausted(process.argv[3]);
    process.stdout.write('cleared\n');
  } else if (cmd === 'sticky') {
    const sub = process.argv[3] || 'get';
    const sid = process.argv[4];
    if (sub === 'get' && sid) process.stdout.write((stickyGet(sid) || '') + '\n');
    else if (sub === 'set' && sid && process.argv[5]) { stickySet(sid, process.argv[5]); process.stdout.write('set\n'); }
    else if (sub === 'clear') { stickyClear(sid); process.stdout.write('cleared\n'); }
    else if (sub === 'list') {
      const st = readJSON(STICKY, {});
      for (const [k, v] of Object.entries(st)) process.stdout.write(`${k}\t${v.acct}\t${v.last_used}\n`);
    } else { process.stderr.write('usage: node pool.js sticky {get|set|clear|list} [sid] [acct]\n'); process.exit(2); }
  } else {
    process.stderr.write('usage: node pool.js {pick|status|clear [name]|sticky ...}\n');
    process.exit(2);
  }
}
