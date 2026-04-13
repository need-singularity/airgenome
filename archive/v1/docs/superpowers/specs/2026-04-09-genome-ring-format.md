# AG3 Genome Ring Buffer Format (v1)

- **Path**: `/mnt/ramdisk/airgenome/genome.ring` (env: `AG3_RING`)
- **Total size**: `64 + 128 * 65536 = 8,388,672 bytes` (~8 MiB) on AG3 tmpfs (16 GiB).
- **Concurrency**: single writer, multiple readers. Atomicity relies on slot-then-header
  write ordering + `fsync`. Readers always snapshot the header first, then iterate
  backward from `write_idx - 1` (mod `slot_count`).

## Header (64 bytes, little-endian)

| Offset | Size | Field        | Type     | Notes                               |
|--------|------|--------------|----------|-------------------------------------|
| 0      | 4    | `magic`      | bytes    | `"AG3R"`                            |
| 4      | 2    | `version`    | uint16   | `1`                                 |
| 6      | 2    | `slot_size`  | uint16   | `128`                               |
| 8      | 4    | `slot_count` | uint32   | `65536`                             |
| 12     | 4    | `write_idx`  | uint32   | next write slot (mod `slot_count`)  |
| 16     | 4    | `wrap_count` | uint32   | number of full wrap-arounds         |
| 20     | 44   | `reserved`   | zero     | future use                          |

`struct` format: `<4sHHIII44x`

## Slot (128 bytes)

| Offset | Size | Field        | Type      | Notes                                   |
|--------|------|--------------|-----------|-----------------------------------------|
| 0      | 8    | `timestamp`  | f64 LE    | Unix epoch seconds                      |
| 8      | 4    | `pid`        | int32 LE  |                                         |
| 12     | 4    | `name_hash`  | uint32 LE | CRC32 of process name (UTF-8)           |
| 16     | 60   | `genome`     | 15 x f32 BE | matches `gpu_gate_mesh` output format |
| 76     | 4    | `cluster_id` | int32 LE  | `-1` = unclassified                     |
| 80     | 4    | `flags`      | uint32 LE | bit0 anomaly, bit1 forge_candidate, ... |
| 84     | 44   | `reserved`   | zero      | future use                              |

`struct` format: `<d i I 60s i I 44x` (total 128 B)

### Byte map

```
Header (64B)
0      4    6    8         12        16        20                        64
+------+----+----+---------+---------+---------+--------------------------+
|AG3R  |ver |ssz |slot_cnt |write_idx|wrap_cnt |          reserved        |
+------+----+----+---------+---------+---------+--------------------------+

Slot (128B)
0          8    12        16                                   76        80        84                   128
+----------+----+---------+------------------------------------+---------+---------+---------------------+
| ts f64   |pid |name_hash|       genome  (15 * f32 BE, 60B)   |clusterId| flags   |      reserved       |
+----------+----+---------+------------------------------------+---------+---------+---------------------+
```

## Wrap-around semantics

- `write_idx` advances 1 per append; when it hits `slot_count` it wraps to 0 and
  `wrap_count` increments.
- Oldest slot is overwritten silently (no tombstoning).
- Total writes ever = `wrap_count * slot_count + write_idx`.
- Readers requesting the last `n` entries receive
  `min(n, total_written)` slots, newest first.

## Concurrency model

- Writer order: seek -> write slot bytes -> seek(0) -> write header -> `flush` -> `fsync`.
- Readers snapshot header with a single `read(64)` then perform independent seek/reads.
  On tmpfs this is effectively atomic per-slot; torn reads are avoided because the
  header update is the last step of an append.
- Multi-writer is **not** supported; guard with a single writer process (e.g., the
  Mac->ubu shipper or the ubu consumer, not both).

## Sizing guide

- Default 8 MiB gives ~65k events. At 100 events/s that's ~10 min of history.
- To resize: change `slot_count` (and/or `slot_size`) and re-run `init`. Resize is
  **destructive**: all prior data is wiped. Consumers must tolerate header reset
  (detect via `wrap_count` decreasing or `slot_count` change).

## Compatibility with Wave 2 (`gpu_gate_mesh`)

`gpu_gate_mesh.py` emits 15 gate pair scores as float32 big-endian, 60 bytes per
process. That payload is written verbatim into the `genome` field, so no
transformation is required between Wave 1 (ring) and Wave 2 (mesh).
