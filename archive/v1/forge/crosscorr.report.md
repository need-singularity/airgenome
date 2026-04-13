# Genome 6-Axis Crosscorrelation Report

N = 2978 genomes · bins = 8

| # | Pair | Pearson r | NMI |
|---|------|-----------|-----|
| 1 | CPU-RAM | +0.1276 | 0.1257 |
| 2 | CPU-Swap | +0.3173 | 0.1094 |
| 3 | CPU-Net | +0.1562 | 0.0445 |
| 4 | CPU-Disk | +0.5813 | 0.1766 |
| 5 | CPU-GPU | +0.4003 | 0.2432 |
| 6 | RAM-Swap | +0.3882 | 0.3002 |
| 7 | RAM-Net | +0.0888 | 0.0850 |
| 8 | RAM-Disk | +0.1493 | 0.1006 |
| 9 | RAM-GPU | +0.2469 | 0.1282 |
| 10 | Swap-Net | +0.1369 | 0.1038 |
| 11 | Swap-Disk | +0.3734 | 0.1226 |
| 12 | Swap-GPU | +0.1768 | 0.0472 |
| 13 | Net-Disk | +0.3002 | 0.0645 |
| 14 | Net-GPU | +0.2068 | 0.0909 |
| 15 | Disk-GPU | +0.3508 | 0.2194 |

## Top 5 by |r|

- 1. **CPU-Disk** — r=+0.5813, NMI=0.1766
- 2. **CPU-GPU** — r=+0.4003, NMI=0.2432
- 3. **RAM-Swap** — r=+0.3882, NMI=0.3002
- 4. **Swap-Disk** — r=+0.3734, NMI=0.1226
- 5. **Disk-GPU** — r=+0.3508, NMI=0.2194

## n=6 Global Analysis

- **det(R)** = 0.332448 (1.0=fully independent, 0.0=degenerate)
- **mean |r|** = 0.2667

### Per-axis coupling

| Axis | mean |r| |
|------|----------|
| CPU | 0.3165 |
| RAM | 0.2002 |
| Swap | 0.2785 |
| Net | 0.1778 |
| Disk | 0.3510 |
| GPU | 0.2763 |

### Axis clusters (|r|>35%)

- **Cluster 1**: CPU+RAM+Swap+Disk+GPU
- **Cluster 2**: Net
