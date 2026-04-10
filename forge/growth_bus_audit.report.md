# growth_bus.jsonl Audit Report

- Date: 2026-04-09
- Target: `$HOME/Dev/nexus/shared/growth_bus.jsonl`
- Total lines: **26,706**
- Sample window: tail 200 lines
- Scope: schema consistency, duplicates, required fields (`ts`, `source`, `event`), airgenome-origin quality

## 1. Source distribution (full file, top)

| count | source |
|------:|:-------|
| 25,613 | blowup-recurse |
|    449 | n6_map_live |
|    323 | absorb_reality |
|    184 | _(missing `source` field)_ |
|     73 | gap_finder_dfs |
|     12 | gap_finder_bridge |
|      1 | airgenome/gpu_cross_sweep |
|      1 | anima |
|     ~8 | tecs-l/*.py (one-shots) |
|      1 | `<PARSE_ERR>` (malformed JSON line) |

## 2. Sample (tail-200) field union

| count | field |
|------:|:------|
| 186 | source |
| 185 | phase, timestamp |
| 184 | type |
| 182 | id, value, grade, confidence, domain, is_axiom |
|  15 | ts |
|  12 | detector |

Essential-field coverage in sample:
- `ts`     missing in **185 / 200** (92.5%)
- `source` missing in **14 / 200** (7.0%)
- `event`  missing in **200 / 200** (100%)

blowup-recurse dominant schema uses `timestamp` (not `ts`) and carries no `event`. It is the de-facto primary schema but does **not** align with what airgenome writers emit.

## 3. airgenome-origin records (full file)

Filter: `source` starts with `airgenome` / `detector` OR `domain == "airgenome"`.

Total: **9 records** (0.03% of bus).

| count | origin |
|------:|:-------|
| 8 | `<no source>` + `domain=airgenome` |
| 1 | `airgenome/gpu_cross_sweep` |

airgenome field union:

| count | field |
|------:|:------|
| 9 | domain |
| 8 | type, ts, detail |
| 7 | title, impact |
| 1 | source, event, metric |
| 1 | phase, timestamp, session, prime_directive, kill_count, kill_targets, kill_excluded |

`ts` formats in airgenome records:
- `date-only` ("2026-04-08"): 7
- `iso8601Z`  ("2026-04-08T22:40:00Z"): 1
- missing: 1

airgenome essential-field coverage:
- `source` missing in **8 / 9**
- `event`  missing in **8 / 9**
- `ts`     missing in **1 / 9**

No duplicate `ts` among airgenome records. No type mismatches (all `ts` are strings where present), but the format is inconsistent.

## 4. Observed anomalies

1. **Two parallel schemas** in the bus. `blowup-recurse` (≈96% of rows) uses `{type,id,value,grade,confidence,domain,is_axiom,phase,timestamp,source}`; airgenome/nexus discoveries use `{type,ts,domain,detail,title,impact}`. No single record type, no versioning field.
2. **`source` missing in 184 rows** file-wide. All are airgenome/nexus "discovery/convergence" records written without the `source` key — the writer(s) rely on `domain=airgenome` instead.
3. **`event` field absent almost everywhere** (200/200 in sample, 8/9 airgenome). If `event` is intended as a required identifier, enforcement is 0%.
4. **`ts` vs `timestamp` collision**. `blowup-recurse` uses `timestamp`, airgenome writers use `ts`. Neither reader can uniformly order the bus.
5. **`ts` format inconsistency** in airgenome rows: mostly `"2026-04-08"` date-only (not sortable to the second), one proper ISO-8601 Z. Epoch seconds (1775714xxx) also appear elsewhere in sample (blowup-recurse phase "1775714119" treated as ts).
6. **Duplicate `ts` (sample)**: 3 distinct ts values repeated (5x, 2x, 6x). Not necessarily errors (batch append), but no dedup key exists.
7. **One malformed JSON line** (`<PARSE_ERR>`) present in the full file — reader must be tolerant.

## 5. Top-5 anomalies (action priority)

1. No unified schema / no `schema_version` field — blowup-recurse vs airgenome-discovery divergence.
2. `source` missing in 184 rows (airgenome/nexus writers omit it).
3. `event` effectively unused (0% sample, 11% airgenome) despite being a candidate required field.
4. `ts` vs `timestamp` naming collision across writers; mixed formats (date-only, iso8601Z, epoch-as-string).
5. One malformed JSON line in the bus.

## 6. Recommendation (non-intrusive)

- Introduce a canonical append helper in `modules/growth_bus_sync.hexa`:
  `gb_validate(record)` checking required fields `{ts:iso8601Z, source:str, event:str, domain:str}` and `gb_append(record)` that normalizes `timestamp→ts` and injects `source` if absent.
- Do **not** modify existing writers in this pass — helper-only, per task instructions.
- Downstream writers can migrate incrementally by calling the helper; the bus file itself stays untouched.
