#!/usr/bin/env python3
"""
plot_monitor.py — visualize the dispatcher monitor CSV.

Auto-detects which simulation produced the CSV:
  - FIFO version (option1.rs) writes one queue column:  queue_len
  - Two-queue version (option2.rs) writes three:        queue_io, queue_cpu, queue_total

Usage:
    python plot_monitor.py monitor_log.csv

Always saves to monitor_plot.png in the current directory.
"""

from __future__ import annotations

import sys
from pathlib import Path

import pandas as pd
import matplotlib
matplotlib.use("Agg")  # no display in Codespaces — render straight to file
import matplotlib.pyplot as plt


OUT_PATH = Path("monitor_plot.png")
POOL_SIZE = 8
CPU_CAP = 100


def detect_queue_columns(df: pd.DataFrame) -> dict:
    """Return a dict telling us which queue layout we have."""
    cols = set(df.columns)
    if {"queue_io", "queue_cpu", "queue_total"}.issubset(cols):
        return {"layout": "two_lane", "io": "queue_io", "cpu": "queue_cpu", "total": "queue_total"}
    if "queue_len" in cols:
        return {"layout": "single", "total": "queue_len"}
    raise SystemExit(
        f"CSV {sorted(cols)} doesn't have the expected queue columns. "
        "Expected either `queue_len` or `queue_io,queue_cpu,queue_total`."
    )


def plot(csv_path: Path) -> None:
    df = pd.read_csv(csv_path)
    if "elapsed_ms" not in df.columns:
        raise SystemExit(f"CSV missing `elapsed_ms` column. Got: {list(df.columns)}")

    layout = detect_queue_columns(df)

    t = df["elapsed_ms"] / 1000.0
    runtime_s = t.iloc[-1] if len(t) else 0.0
    title = f"{csv_path.name}  (runtime {runtime_s:.1f}s)"

    fig, (ax_cpu, ax_workers, ax_queue) = plt.subplots(
        3, 1, figsize=(13, 9), sharex=True
    )
    fig.suptitle(title)

    # ── Panel 1: CPU usage ─────────────────────────────────────
    avg_cpu = df["cpu_pct"].mean()
    ax_cpu.plot(t, df["cpu_pct"], color="C0", linewidth=0.9)
    ax_cpu.axhline(CPU_CAP, color="red", linestyle="--", linewidth=1, label=f"cap ({CPU_CAP}%)")
    ax_cpu.axhline(avg_cpu, color="gray", linestyle=":", linewidth=1, label=f"avg ({avg_cpu:.1f}%)")
    ax_cpu.set_ylabel("CPU usage (%)")
    ax_cpu.set_ylim(0, max(CPU_CAP + 5, df["cpu_pct"].max() + 5))
    ax_cpu.legend(loc="upper right")
    ax_cpu.grid(True, alpha=0.3)

    # ── Panel 2: workers active ────────────────────────────────
    avg_workers = df["workers_active"].mean()
    ax_workers.plot(t, df["workers_active"], color="C2", linewidth=0.9)
    ax_workers.axhline(POOL_SIZE, color="red", linestyle="--", linewidth=1, label=f"pool size ({POOL_SIZE})")
    ax_workers.axhline(avg_workers, color="gray", linestyle=":", linewidth=1, label=f"avg ({avg_workers:.2f})")
    ax_workers.set_ylabel("workers active")
    ax_workers.set_ylim(0, POOL_SIZE + 0.5)
    ax_workers.legend(loc="upper right")
    ax_workers.grid(True, alpha=0.3)

    # ── Panel 3: queue length(s) ───────────────────────────────
    if layout["layout"] == "single":
        peak = df[layout["total"]].max()
        ax_queue.plot(t, df[layout["total"]], color="C1", linewidth=0.9, label=f"queue (peak {peak})")
        ax_queue.axhline(peak, color="gray", linestyle=":", linewidth=1)
    else:
        # Two-lane: IO and CPU as bold lines, total as faint gray for context.
        # The story is "are the lanes balanced or is one starving?"
        peak_io = df[layout["io"]].max()
        peak_cpu = df[layout["cpu"]].max()
        peak_total = df[layout["total"]].max()
        ax_queue.plot(t, df[layout["total"]], color="gray", linewidth=0.8, alpha=0.5,
                      label=f"total (peak {peak_total})")
        ax_queue.plot(t, df[layout["io"]], color="C0", linewidth=1.1,
                      label=f"IO queue (peak {peak_io})")
        ax_queue.plot(t, df[layout["cpu"]], color="C3", linewidth=1.1,
                      label=f"CPU queue (peak {peak_cpu})")

    ax_queue.set_ylabel("queue length")
    ax_queue.set_xlabel("elapsed time (s)")
    ax_queue.legend(loc="upper right")
    ax_queue.grid(True, alpha=0.3)

    plt.tight_layout(rect=[0, 0, 1, 0.97])
    plt.savefig(OUT_PATH, dpi=120)
    print(f"saved plot -> {OUT_PATH}")


def main() -> None:
    if len(sys.argv) != 2:
        sys.exit("usage: python plot_monitor.py <csv>")
    csv_path = Path(sys.argv[1])
    if not csv_path.exists():
        sys.exit(f"CSV not found: {csv_path}")
    plot(csv_path)


if __name__ == "__main__":
    main()