# MacBook ↔ Ubuntu 자원 브릿지

동일 네트워크 + USB 직결 환경에서 Ubuntu의 CPU/RAM/디스크를 MacBook이 끌어쓰는 구성.
API 호출 없이, 비정식 경로 포함.

---

## 1단계: USB 직결 네트워크

MacBook ↔ Ubuntu를 USB-C/Thunderbolt 케이블로 직연결 → RNDIS 또는 CDC ECM으로 가상 이더넷.

### Ubuntu 쪽

```bash
# USB gadget 이더넷 확인
ip link show  # usb0 같은 인터페이스 확인

# 고정 IP 할당
sudo ip addr add 10.0.0.2/24 dev usb0
sudo ip link set usb0 up
```

### MacBook 쪽

시스템 설정 → 네트워크에 USB 이더넷 잡히면:

```
10.0.0.1/24  # 수동 설정
```

결과: **10.0.0.1 ↔ 10.0.0.2** 전용 링크 — Wi-Fi 안 거치고 직통.

---

## 2단계: SSH 고속 파이프

```bash
# MacBook ~/.ssh/config
Host ubu
    HostName 10.0.0.2
    User ghost
    ControlMaster auto
    ControlPath ~/.ssh/ctl-%r@%h:%p
    ControlPersist 10m
    Compression no          # USB 직결이라 압축 불필요
```

`ssh ubu` 한 번 연결하면 이후 세션은 멀티플렉싱 — 연결 오버헤드 제로.

---

## 3단계: 자원 끌어쓰기

### CPU 오프로드 (원격 실행)

```bash
# MacBook에서 Ubuntu CPU로 작업 위임
ssh ubu 'cd /tmp && cargo build --release'

# 결과만 가져오기
scp ubu:/tmp/target/release/binary ./
```

### RAM/디스크 확장 (sshfs)

```bash
# Ubuntu 파일시스템을 MacBook에 마운트
mkdir -p ~/mnt/ubu
sshfs ubu:/home/ghost ~/mnt/ubu -o auto_cache,reconnect

# 이제 ~/mnt/ubu 가 Ubuntu 디스크
```

### netcat 파이프 (프로토콜 오버헤드 zero)

```bash
# Ubuntu (수신)
nc -l 9999 | bash  # 받은 명령 즉시 실행

# MacBook (송신)
echo "ps aux --sort=-%mem | head -20" | nc 10.0.0.2 9999
```

### 메모리 직접 공유 (Ubuntu RAM → MacBook 마운트)

```bash
# Ubuntu에서 tmpfs를 NFS로 노출
sudo mount -t tmpfs -o size=8G tmpfs /mnt/shared
sudo exportfs -o rw,no_root_squash,insecure 10.0.0.1:/mnt/shared

# MacBook에서 마운트
sudo mount -t nfs 10.0.0.2:/mnt/shared ~/mnt/shared
```

Ubuntu RAM 8GB를 MacBook 로컬 디스크처럼 사용 — SSD보다 빠름.

---

## airgenome 연동

```
MacBook (genome 수집) ──USB 직결──→ Ubuntu (연산 오프로드)
     ↑                                    │
     └──── 결과 genome/시그니처 반환 ←────┘
```

- **MacBook**: 센서 수집 + 게이트 투영 (가벼움)
- **Ubuntu**: 무거운 연산 (빌드, 시그니처 분석, 누적 패턴 추출)

---

## 방식 요약

| 방식 | 설명 | 용도 |
|---|---|---|
| SSH 원격 실행 | 명령 위임 + 결과 반환 | CPU 오프로드 |
| sshfs 마운트 | Ubuntu 디스크를 로컬처럼 | 스토리지 확장 |
| distcc | 컴파일 분산 | 빌드 가속 |
| netcat 파이프 | raw TCP, 오버헤드 제로 | 실시간 스트림 |
| NFS tmpfs | Ubuntu RAM 공유 | 초고속 캐시 |
| 리버스 터널 | NAT 우회 | 외부망 접근 시 |
| USB 직결 | 물리 케이블, 지연 최소 | 주 링크 |

---

## 환경 조건

- 동일 네트워크
- USB 직결 가능
- Prime Directive 준수: 양쪽 모두 프로세스 kill 없이 여유 자원만 사용
