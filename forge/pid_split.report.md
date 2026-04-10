# pid_split Report

- input: `forge/genomes.index.jsonl`
- out_dir: `forge/by_pid`
- unique pids: 50
- total records: 100
- length min/mean/max: 2 / 2.00 / 2

## Top 10 pids by sample count

| rank | pid | samples |
|---|---|---|
| 1 | 169 | 2 |
| 2 | 168 | 2 |
| 3 | 167 | 2 |
| 4 | 165 | 2 |
| 5 | 164 | 2 |
| 6 | 163 | 2 |
| 7 | 162 | 2 |
| 8 | 161 | 2 |
| 9 | 160 | 2 |
| 10 | 159 | 2 |

## τ=50 Artifact Verification

샘플 pid: **101** (`forge/by_pid/101.jsonl`)

```
$ $HEXA modules/genome_crosscorr.hexa --input forge/by_pid/101.jsonl
  genome_crosscorr — loading forge/by_pid/101.jsonl
  rows loaded: 2
  ERROR: too few rows
```

### 해석

- 현재 `forge/genomes.index.jsonl` 은 총 100 레코드, unique pid=50, pid 당 N=2 (min=mean=max=2).
- `pid_split` 후 단일 pid 시계열 길이가 2 이므로 τ=50 은 **정의 불가능** (τ_max = N−1 = 1) → τ=50 자기상관 스파이크는 **구조적으로 소멸**.
- 이는 `modules/alias_probe.hexa` 결론과 일치: τ=50 스파이크는 샘플러 round-robin (pids[0:50]==pids[50:100]) 으로 인한 self-revisit 이며, pid-local 시계열에는 존재할 수 없다.
- 실데이터 MI-vs-τ 스펙트럼 비교는 pid 당 N≫50 이 축적된 후 가능. 권장: sampler 가 동일 pid 를 ≥64회 기록할 때까지 수집한 뒤 재실행:

```
$HEXA modules/pid_split.hexa
$HEXA modules/genome_crosscorr.hexa --input forge/by_pid/<pid>.jsonl
$HEXA modules/detectors/run_all.hexa --input forge/by_pid/<pid>.jsonl
```

### 사용법 요약

- `modules/pid_split.hexa` — `forge/genomes.index.jsonl` → `forge/by_pid/<pid>.jsonl` 분리 (ts 오름차순).
  - 옵션: `--input <path>`, `--out-dir <dir>`, `--report <file>`
- `modules/genome_crosscorr.hexa --input <by_pid_file>` — 단일 pid 시계열로 15쌍 Pearson/NMI 재계산.
- `modules/detectors/run_all.hexa --input <by_pid_file>` — `detectors.jsonl` 의 `genomes_index` 를 임시 override 후 6종 detector 실행, 종료 시 자동 복원.
