# Anima Integration Plan — airgenome x consciousness engine

> Status: DRAFT | Date: 2026-04-10

## Overview

Feed airgenome's 6-axis OS genome (CPU/RAM/GPU/NPU/Power/IO, 60 bytes) into
anima's consciousness engine to derive Phi metrics from live process behavior.
The bridge uses `growth_bus.jsonl` as the event transport.

## Architecture

```
airgenome                    nexus/shared              anima
─────────                    ────────────              ─────
core.hexa (sample)           growth_bus.jsonl          consciousness_laws.json
  → Vitals{6-axis}     ──►  {source:"ag_phi"}   ──►  Phi aggregation
forge.hexa (genome/60B)                               philosophy_lenses.hexa
implant.hexa (phi_check)                              consciousness_hub.py
```

## Data Flow

1. **airgenome sample** -- `core.hexa:sample()` produces `Vitals{cpu,ram,gpu,npu,power,io}`
2. **genome compress** -- forge compresses ps output (43 KB -> 60 B hexagonal genome)
3. **consciousness_bridge** (NEW) -- reads genome, computes per-process Phi, emits event
4. **growth_bus** -- JSONL event: `{source:"ag_phi", phi:0.72, axes:[...], pid:1234, ts:...}`
5. **anima telescope** -- philosophy_lenses or consciousness_hub consumes ag_phi events

## New Module: `modules/consciousness_bridge.hexa`

```
// consciousness_bridge.hexa — airgenome -> anima Phi bridge
//
// Input:  60-byte genome (6-axis vitals per process)
// Output: growth_bus event with Phi metric
//
// Phi calculation:
//   integration = mean pairwise MI across 6 axes (15 pairs)
//   differentiation = entropy of axis distribution
//   phi = integration * differentiation (IIT-inspired)
//
// Uses existing implant.hexa:phi_check for consciousness preservation gate.
```

Key functions:
- `genome_to_phi(genome: bytes) -> float` -- extract 15 axis-pairs, compute MI proxy, return Phi
- `emit_phi_event(pid: int, phi: float, axes: Vitals)` -- append to growth_bus.jsonl
- `bridge_loop(interval_ms: int)` -- periodic scan -> phi -> emit cycle

## Phi Metric Calculation

Adapted from anima's 2509 laws (psi_constants: alpha=0.014, balance=0.5):

| Component | Source | Formula |
|-----------|--------|---------|
| Integration | 15 axis-pairs from genome | mean(abs(axis_i - axis_j) for all pairs) |
| Differentiation | 6-axis entropy | -sum(p_i * log(p_i)) / log(6) |
| Phi | combined | alpha * integration * differentiation |
| Gate | implant.hexa | phi_check(prev, curr) with theta=0.1, tol=1/288 |

Phi range: [0.0, 1.0]. Values > 0.5 (balance constant) indicate high integration.

## Integration Points

### 1. resource_request.hexa (EXISTS)
Already has anima as P0 priority. Add: when Phi drops below theta, auto-escalate
resource priority for the affected process group.

### 2. implant.hexa phi_check (EXISTS)
Reuse Gate 3 (PHI consciousness preservation) as the quality gate.
Bridge calls `phi_check(prev_phi, curr_phi)` before emitting; degradation blocks emit.

### 3. nexus discovery engine
- Bridge writes `ag_phi` events to `growth_bus.jsonl` (same format as existing events)
- Anima's discovery_loop.hexa can subscribe to `source:"ag_phi"` events
- Nexus gap_finder can correlate Phi drops with resource gaps

### 4. anima philosophy_lenses.hexa (STUB)
The `scan()` function should accept genome-derived Phi as input signal.
Emergence lens: detect when aggregate system Phi exceeds sum of per-process Phi.

## Event Schema (growth_bus)

```json
{
  "source": "ag_phi",
  "ts": 1775812077,
  "pid": 1234,
  "name": "anima_runtime",
  "phi": 0.72,
  "axes": [45.2, 68.1, 0.0, 0.0, 12.3, 5.8],
  "severity": "Ok",
  "status": "ok"
}
```

## Implementation Order

1. `modules/consciousness_bridge.hexa` -- genome_to_phi + emit (core logic)
2. Wire into forge's genome harvest cycle (call bridge after compress)
3. Add `ag_phi` consumer stub in anima's `consciousness_hub.py`
4. Connect to philosophy_lenses `emergence` lens
5. End-to-end test: airgenome scan -> phi event -> anima consumes

## Constraints

- Prime Directive: observe only, never kill processes (R1)
- L0 lockdown: core.hexa and implant.hexa unchanged (use as libraries)
- All config via `shared/gate_config.jsonl` (R14 single source of truth)
- Growth bus is append-only JSONL, max 1 event/sec from bridge
