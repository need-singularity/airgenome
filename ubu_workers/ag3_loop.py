#!/usr/bin/env python3
"""
AG3 연속 돌파 루프
- 게놈 링 폴링 → 이상치 감지 → growth_bus append → blowup 트리거
- GPU 유휴 감지 → forge 자동 가동
"""
import time, json, os, subprocess, sys, struct
sys.path.insert(0, os.path.expanduser("~/airgenome/ag3"))
import ring_io  # W1-X에서 작성됨

CFG_PATH = os.environ.get("AG3_CONFIG", os.path.expanduser("~/airgenome/nexus/shared/gate_config.jsonl"))
GROWTH_BUS = os.path.expanduser("~/Dev/nexus/shared/growth_bus.jsonl")
POLL_SEC = 5
IDLE_THRESHOLD_SEC = 300
ANOMALY_SIM_LOW = 0.3  # top-1 이웃 유사도 < 0.3 → 이상치

def load_cfg():
    cfg = {}
    try:
        with open(CFG_PATH) as f:
            for line in f:
                try:
                    j = json.loads(line)
                    cfg[j["key"]] = j["value"]
                except: pass
    except: pass
    return cfg

def gpu_idle_sec():
    try:
        r = subprocess.run(["nvidia-smi","--query-gpu=utilization.gpu","--format=csv,noheader"],
                           capture_output=True, text=True, timeout=3)
        util = int(r.stdout.strip().rstrip("%"))
        return util
    except: return 100

def append_growth(event):
    with open(GROWTH_BUS, "a") as f:
        f.write(json.dumps(event) + "\n")

def trigger_blowup(cfg, domain="resource"):
    hexa = cfg.get("ubu_hexa_bin", "/home/aiden/Dev/hexa-lang/target/release/hexa")
    blowup = cfg.get("ubu_blowup_hexa", "/home/aiden/Dev/nexus/mk2_hexa/native/blowup.hexa")
    try:
        subprocess.Popen([hexa, blowup, domain, "3", "--no-graph"],
                         stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        return True
    except: return False

def main():
    cfg = load_cfg()
    print(f"[AG3-LOOP] started. cfg_keys={len(cfg)}")
    last_write_idx = -1
    idle_start = None
    while True:
        try:
            h = ring_io.read_header()
            if h["write_idx"] != last_write_idx:
                last_write_idx = h["write_idx"]
                recent = list(ring_io.iter_recent(10))
                for r in recent:
                    if r["flags"] & 1:
                        append_growth({
                            "type": "anomaly_observed", "phase": "ag3",
                            "id": f"pid_{r['pid']}", "value": 1,
                            "grade": "INFO", "domain": "resource",
                            "timestamp": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime())
                        })
                        if trigger_blowup(cfg):
                            print(f"[AG3-LOOP] blowup triggered for pid {r['pid']}")
            util = gpu_idle_sec()
            if util < 5:
                if idle_start is None:
                    idle_start = time.time()
                elif time.time() - idle_start > IDLE_THRESHOLD_SEC:
                    append_growth({
                        "type": "gpu_idle_forge", "phase": "ag3",
                        "id": "idle_forge", "value": int(time.time() - idle_start),
                        "grade": "INFO", "domain": "resource",
                        "timestamp": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime())
                    })
                    idle_start = time.time()
            else:
                idle_start = None
        except Exception as e:
            print(f"[AG3-LOOP] err: {e}", file=sys.stderr)
        time.sleep(POLL_SEC)

if __name__ == "__main__":
    main()
