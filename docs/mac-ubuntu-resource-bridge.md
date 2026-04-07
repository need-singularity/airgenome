# MacBook ↔ Ubuntu 자원 브릿지

동일 네트워크에서 Ubuntu의 CPU/RAM/디스크를 MacBook이 끌어쓰는 구성.
API 호출 없이, 비정식 경로 포함.

---

## 현재 구성 (Wi-Fi, 2026-04-07)

| 항목 | 값 |
|---|---|
| MacBook IP | 192.168.132.82 |
| Ubuntu IP | 192.168.50.119 |
| Ubuntu 호스트 | aiden-B650M-K |
| Ubuntu CPU | AMD Ryzen 5 9600X (6코어/12스레드) |
| Ubuntu RAM | 30GB (28GB 가용) |
| Ubuntu 디스크 | 915GB NVMe (853GB 여유) |
| Ubuntu 온보드 NIC | Realtek RTL8125 2.5GbE |
| 연결 방식 | Wi-Fi SSH (지연 ~3.5ms) |
| SSH alias | `ssh ubu` |
| SSH user | aiden |

### 벤치마크 결과

| 벤치 | MacBook | Ubuntu | Ubuntu 배수 |
|---|---|---|---|
| CPU (10M sum) | 1.013s | 0.227s | **4.5x 빠름** |
| RAM (5M list) | 0.213s | 0.053s | **4.0x 빠름** |

---

## 네트워크 업그레이드 경로

### 속도 순위

| 순위 | 방식 | 속도 | 지연 | 비용 |
|---|---|---|---|---|
| 1 | 10GbE (X550-T1) | 10Gbps | 0.1ms | ~15만원 |
| 2 | 온보드 2.5G 랜선 직결 | 2.5Gbps | 0.5ms | 동글 ~2만원 |
| 3 | Wi-Fi (현재) | 200-800Mbps | 3.5ms | 0원 |

### 옵션 1: 10GbE 업그레이드

#### Ubuntu 쪽 — PCIe 10GbE NIC

| 순위 | 카드 | 칩셋 | Ubuntu 드라이버 | 가격 | 비고 |
|---|---|---|---|---|---|
| **1** | **TP-Link TX401** | Marvell AQC113 | 커널 내장 (atlantic) | ~5만원 | 가성비 최고, 케이블 포함 |
| **2** | **Intel X550-T1** | Intel X550 | 커널 내장 (ixgbe), Ubuntu 공식 인증 | ~15만원 | 안정성 최고, 발열 큼 |
| 비추 | ASUS XG-C100C | Marvell AQC107 | 수동 컴파일 필요 가능 | ~7만원 | 번거로움 |
| 비추 | Mellanox ConnectX-3 | Mellanox | Ubuntu 24.04 드라이버 설치 실패 보고 | ~5-8만원 | 최신 커널 충돌 |

**TX401 vs X550-T1 성능 비교:**

| 항목 | TX401 (AQC113) | X550-T1 |
|---|---|---|
| 실측 처리량 | 9-10 Gbps | 9.9 Gbps |
| 소비전력 | 4W | 7-9W (발열 많음) |
| CPU 오프로드 | 기본만 | TSO/LRO/RSS 풀 오프로드 |
| 지속 부하 안정성 | 간헐적 끊김 보고 | 매우 안정 |
| 드라이버 성숙도 | 역사 짧음 | 10년+ 검증 |
| AMD CPU 호환 | O | O (PCIe 표준 카드) |

- SSH 오프로드 용도 → TX401 충분
- 대용량 지속 전송/NFS 마운트 → X550-T1 추천

**국내 구매 (네이버 쇼핑 기준):**

| 제품 | 가격 |
|---|---|
| 넥시 NX-X550-T1 (NX545) | 156,390원 (최저, 리뷰 5.0) |
| 서버용 호환 X550-T1 | 181,600원 |
| 랜스타 LS-X550-T1 | 253,920원 |
| 인텔 정품 X550-T1 | 426,100원 |

#### MacBook 쪽 — USB-C/TB to 10GbE 어댑터

| 어댑터 | 연결 | 가격 |
|---|---|---|
| **SABRENT USB4 10GbE** | USB4/TB | ~10만원 (가성비) |
| **OWC Thunderbolt 10G** | TB3/4 | ~15만원 (안정적) |
| CalDigit Connect 10G | TB3/4/5 | ~13만원 |
| Ubiquiti USB-C 10GbE | USB-C | ~25만원 |

USB-C to Ethernet 동글은 Thunderbolt 미지원이어도 상관없음 — USB 3.0 프로토콜로 작동, 2.5Gbps는 USB 3.0 (5Gbps) 대역폭 안에 들어감.

#### 케이블

- RJ45: CAT6A (~5천원), 직결이면 1m 충분
- 10GbE 추천 조합: TX401 + SABRENT USB4 = ~15만원 총액

### 옵션 2: 온보드 2.5G 랜선 직결

Ubuntu에 이미 Realtek RTL8125 2.5GbE 있음. MacBook에 USB-C to Ethernet 동글만 추가하면 됨.

```bash
# MacBook에 동글 연결 후
# 수동 IP 설정 (동일 서브넷)
# MacBook: 10.0.0.1/24
# Ubuntu:  10.0.0.2/24
```

동글 가격: ~2만원. Wi-Fi 대비 5-10배 빠르고 지연 0.5ms.

### Thunderbolt 직결 — 불가

- Ubuntu B650M에 TB 포트 없음
- AMD 메인보드에 TB PCIe 카드 장착 불가 (Intel 전용)
- USB4 PCIe 카드 (ASRock USB4 AIC) → 리눅스 드라이버 불안정 리스크

---

## 현재 SSH 설정

```bash
# MacBook ~/.ssh/config
Host ubu
    HostName 192.168.50.119
    User aiden
    ControlMaster auto
    ControlPath ~/.ssh/ctl-%r@%h:%p
    ControlPersist 10m
    Compression no
```

---

## 자원 끌어쓰기 방법

### CPU 오프로드 (원격 실행)

```bash
ssh ubu 'cd /tmp && cargo build --release'
scp ubu:/tmp/target/release/binary ./
```

### RAM/디스크 확장 (sshfs)

```bash
mkdir -p ~/mnt/ubu
sshfs ubu:/home/aiden ~/mnt/ubu -o auto_cache,reconnect
```

### netcat 파이프 (프로토콜 오버헤드 zero)

```bash
# Ubuntu (수신)
nc -l 9999 | bash

# MacBook (송신)
echo "ps aux --sort=-%mem | head -20" | nc 192.168.50.119 9999
```

### Ubuntu RAM 공유 (NFS tmpfs)

```bash
# Ubuntu
sudo mount -t tmpfs -o size=8G tmpfs /mnt/shared
sudo exportfs -o rw,no_root_squash,insecure <MacBook_IP>:/mnt/shared

# MacBook
sudo mount -t nfs 192.168.50.119:/mnt/shared ~/mnt/shared
```

---

## airgenome offload 모듈

```bash
HEXA=~/Dev/hexa-lang/target/release/hexa

$HEXA run mk2_hexa/native/offload.hexa status         # Ubuntu 상태
$HEXA run mk2_hexa/native/offload.hexa bench           # 성능 비교
$HEXA run mk2_hexa/native/offload.hexa genome-remote   # Ubuntu 프로세스 게놈
$HEXA run mk2_hexa/native/offload.hexa exec "ls -la"   # 원격 명령
$HEXA run mk2_hexa/native/offload.hexa sync            # 로그 동기화
```

---

## 환경 조건

- 동일 네트워크 (Wi-Fi)
- USB 직결 가능 (미사용)
- Prime Directive 준수: 양쪽 모두 프로세스 kill 없이 여유 자원만 사용
