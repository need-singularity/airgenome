#!/usr/bin/env python3
"""
AG3 6축 게이트 메쉬 GPU 병렬 워커
- 입력: stdin binary (N×6 float32 little-endian) or --arrow <path>
- 출력: stdout JSONL {"idx","genome","gates"[15]}
- 15쌍 순서: (0,1)(0,2)(0,3)(0,4)(0,5)(1,2)(1,3)(1,4)(1,5)(2,3)(2,4)(2,5)(3,4)(3,5)(4,5)
- 게놈: 15 × float32 big-endian = 60바이트 → hex
"""
import sys
import json
import struct
import argparse
import torch

PAIR_I = [0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 3, 3, 4]
PAIR_J = [1, 2, 3, 4, 5, 2, 3, 4, 5, 3, 4, 5, 4, 5, 5]


def load_input(args):
    if args.arrow:
        try:
            import pyarrow as pa  # noqa: F401
            import pyarrow.ipc as ipc
        except ImportError:
            sys.stderr.write("[gpu_gate_mesh] pyarrow unavailable, fallback stdin\n")
        else:
            with open(args.arrow, "rb") as f:
                reader = ipc.open_file(f) if args.arrow.endswith(".arrow") else ipc.open_stream(f)
                tbl = reader.read_all()
            arr = tbl.to_pandas().to_numpy(dtype="float32")
            assert arr.shape[1] == 6, f"expected 6 cols, got {arr.shape}"
            return torch.from_numpy(arr)
    raw = sys.stdin.buffer.read()
    N = len(raw) // (6 * 4)
    if N == 0:
        sys.stderr.write("[gpu_gate_mesh] empty stdin\n")
        sys.exit(0)
    return torch.frombuffer(bytearray(raw), dtype=torch.float32).view(N, 6)


def pick_device(requested: str) -> str:
    if requested != "cuda":
        return requested
    if not torch.cuda.is_available():
        sys.stderr.write("[gpu_gate_mesh] CUDA unavailable -> CPU fallback\n")
        return "cpu"
    try:
        free, total = torch.cuda.mem_get_info()
        if free < (1 << 30):
            sys.stderr.write(
                "[gpu_gate_mesh] WARN low VRAM free=%.2fGB -> CPU fallback\n" % (free / 1e9)
            )
            return "cpu"
    except Exception as e:
        sys.stderr.write("[gpu_gate_mesh] mem_get_info failed: %s\n" % e)
    return "cuda"


def compute_gates(x: torch.Tensor, device: str) -> torch.Tensor:
    I = torch.tensor(PAIR_I, device=device, dtype=torch.long)
    J = torch.tensor(PAIR_J, device=device, dtype=torch.long)
    x = x.to(device)
    xn = x / (x.norm(dim=1, keepdim=True) + 1e-9)
    return xn[:, I] * xn[:, J]


def emit(gates_np, offset: int, out):
    for i in range(gates_np.shape[0]):
        g = [float(v) for v in gates_np[i].tolist()]
        genome = b"".join(struct.pack(">f", v) for v in g).hex()
        out.write(json.dumps({"idx": offset + i, "genome": genome, "gates": g}) + "\n")


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--arrow", help="arrow file path (optional)")
    ap.add_argument("--device", default="cuda")
    ap.add_argument("--chunk", type=int, default=65536)
    ap.add_argument("--vram-report", action="store_true")
    args = ap.parse_args()

    x_all = load_input(args)
    N = x_all.shape[0]
    device = pick_device(args.device)

    if device == "cuda":
        torch.cuda.reset_peak_memory_stats()

    out = sys.stdout
    for start in range(0, N, args.chunk):
        end = min(start + args.chunk, N)
        g = compute_gates(x_all[start:end], device)
        emit(g.detach().cpu().numpy(), start, out)
        del g

    if device == "cuda":
        peak_mb = torch.cuda.max_memory_allocated() / (1024 * 1024)
        if args.vram_report:
            sys.stderr.write("[gpu_gate_mesh] N=%d peak_vram=%.2fMB\n" % (N, peak_mb))
        torch.cuda.empty_cache()


if __name__ == "__main__":
    main()
