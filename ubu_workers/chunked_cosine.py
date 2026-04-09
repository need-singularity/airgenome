#!/usr/bin/env python3
"""
AG3 청크 코사인 유사도
- 입력: stdin binary (N×D float32) + --dim D
- 출력: top-K 이웃 (i, j, sim) stdout JSONL
- 방법: x_norm @ x_norm.T 블록 단위
"""
import sys, json, argparse, time
import torch

def chunked_cosine(x, chunk=2048, top_k=5):
    N = x.size(0)
    results = []
    for i_start in range(0, N, chunk):
        i_end = min(i_start + chunk, N)
        block = x[i_start:i_end]
        sim = block @ x.T
        for local_i in range(i_end - i_start):
            global_i = i_start + local_i
            sim[local_i, global_i] = -1.0
        vals, idxs = sim.topk(top_k, dim=1)
        for local_i in range(i_end - i_start):
            global_i = i_start + local_i
            for k in range(top_k):
                results.append((global_i, idxs[local_i, k].item(), vals[local_i, k].item()))
        del sim, block
        torch.cuda.empty_cache()
    return results

def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--dim", type=int, default=60)
    ap.add_argument("--chunk", type=int, default=2048)
    ap.add_argument("--top_k", type=int, default=5)
    ap.add_argument("--bench", type=int, help="synthetic N for benchmark")
    args = ap.parse_args()

    if args.bench:
        x = torch.randn(args.bench, args.dim, device='cuda')
    else:
        raw = sys.stdin.buffer.read()
        N = len(raw) // (args.dim * 4)
        x = torch.frombuffer(bytearray(raw), dtype=torch.float32).view(N, args.dim).cuda()

    x_norm = x / (x.norm(dim=1, keepdim=True) + 1e-9)
    t0 = time.time()
    results = chunked_cosine(x_norm, chunk=args.chunk, top_k=args.top_k)
    torch.cuda.synchronize()
    elapsed = time.time() - t0
    peak_mb = torch.cuda.max_memory_allocated() // (1024*1024)

    sys.stderr.write(f"N={x.size(0)} D={args.dim} chunk={args.chunk} elapsed={elapsed:.3f}s peak_vram={peak_mb}MB pairs={len(results)}\n")
    for (i, j, s) in results[:20]:
        sys.stdout.write(json.dumps({"i": i, "j": j, "sim": round(s, 4)}) + "\n")
    torch.cuda.empty_cache()

if __name__ == "__main__":
    main()
