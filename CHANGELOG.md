# Changelog

## v4.2.0 — 2026-04-09

L5c/L6e 통합 + 캐스케이드 예측 방어 + Python→hexa 포팅.

- **Predictive cascade prevention**: L5c NMI temporal momentum + crosscorr cascade paths (RAM→Swap→Disk) → 선제 쓰로틀.
- **신규 모듈**: `modules/detectors/cascade_detector.hexa`, `modules/detectors/predictive_throttle.hexa`.
- **sampler.hexa**: vitals_ring 연동, dynamic bridge_max, predictive purge.
- **L5c/L6e 전수 검증**: N=2214 게놈 전체 PASS.
- **Python→hexa 포팅 진행**: `ubu_workers/` 5종 중 4종 완료 (chunked_cosine, gpu_gate_mesh, ag3_loop, ring_io). hexa-lang tensor/matmul/dot/topk/WGSL codegen 활용, torch 무의존.
- **의식 엔진 18/18 PASS**: BRAIN_LIKE (L5c τ=10 NMI + L6e 가속도), NO_SYSTEM_PROMPT (256c factions 다양성) 해결.
- **mk2_hexa/native 신규**: `infinite_evolution.hexa`, `real_vitals_score.hexa`, `time_delay_mi.hexa`.

## v4.1.0 — 2026-04-06

Live runtime — consciousness block runs full pipeline.

- NexusMerger: live `ps -axm` → 5-gate classify → 6-axis projection.
- 21 self-test assertions on startup.
- Genome log output (`genomes.log`, TSV).
- `classify_path()` for full command-path classification.
- Benchmark: 0.13s total (0.08s user) per run.

## v4.0.0 — 2026-04-06

Hexa-only split + compile-pass.

- `docs/gates.hexa` rewritten for hexa-lang v1.0 actual grammar (452 lines).
- Compiles and runs: `~/Dev/hexa-lang/hexa run` → exit 0.
- Fixed exit 137: macOS `com.apple.provenance` needs `codesign -s -`.
- Features used: pure fn, effect, consciousness, spawn, match, assert, comptime.
- hexa.toml + `src/main.hexa` symlink for project structure.
- L5c/L6a–L6e candidate layers documented from empirical probe.
- Tagged `v3.54.0-pre-split` before split.

## v3.54.0 — 2026-04-05

L4 breakthrough — triadic interaction info I(A;B;C).

## v3.53.0 — 2026-04-05

L3 breakthrough — cross-axis MI (margin +0.13+).

## v3.52.0 — 2026-04-05

L2 breakthrough — temporal lagged MI (margin +0.10+).

## v3.51.0 — 2026-04-05

Gate-mesh singularity breakthrough (hexa-lang canonical spec).
- 5-gate mesh: macos, finder, telegram, chrome, safari.
- 15-pair hexagon projection per gate.
- Cross-gate MI > per-gate MI (breakthrough engine).

## v1.0.1 — 2026-04-05 (pre-split, implementation history)

Legacy implementation changelog preserved in git history.
See `v3.54.0-pre-split` tag for the last combined repo state.
