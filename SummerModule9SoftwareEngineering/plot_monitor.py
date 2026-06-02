#!/usr/bin/env python3
"""
Plot the monitor CSV produced by the FIFO dispatcher.

Usage:
    python plot_monitor.py                       # uses ./monitor_log.csv
    python plot_monitor.py path/to/file.csv      # use a different CSV
    python plot_monitor.py file.csv out.png      # custom output filename

Output: a PNG with three stacked panels (CPU%, workers active, queue length)
sharing a time axis. Saves to monitor_plot.png by default.
"""

import sys
import csv
from pathlib import Path
import matplotlib
matplotlib.use("Agg")  # headless backend — works in Codespaces over SSH
import matplotlib.pyplot as plt

# ── parse args ────────────────────────────────────────────────────
csv_path = Path(sys.argv[1]) if len(sys.argv) > 1 else Path("monitor_log.csv")
out_path = Path(sys.argv[2]) if len(sys.argv) > 2 else Path("monitor_plot.png")

if not csv_path.exists():
    sys.exit(f"error: {csv_path} not found. Run the simulation first.")

# ── load the CSV ──────────────────────────────────────────────────
elapsed_s, cpu, workers, qlen = [], [], [], []
with csv_path.open() as f:
    reader = csv.DictReader(f)
    for row in reader:
        elapsed_s.append(int(row["elapsed_ms"]) / 1000.0)  # → seconds
        cpu.append(int(row["cpu_pct"]))
        workers.append(int(row["workers_active"]))
        qlen.append(int(row["queue_len"]))

if not elapsed_s:
    sys.exit("error: CSV has no data rows.")

# ── compute summary lines for annotation ──────────────────────────
avg_cpu = sum(cpu) / len(cpu)
avg_workers = sum(workers) / len(workers)
peak_q = max(qlen)
runtime = elapsed_s[-1]

# ── plot ──────────────────────────────────────────────────────────
fig, axes = plt.subplots(3, 1, figsize=(11, 8), sharex=True)

# CPU usage with the cap line
axes[0].plot(elapsed_s, cpu, linewidth=0.9)
axes[0].axhline(100, color="red", linestyle="--", linewidth=0.8, label="cap (100%)")
axes[0].axhline(avg_cpu, color="gray", linestyle=":", linewidth=0.8,
                label=f"avg ({avg_cpu:.1f}%)")
axes[0].set_ylabel("CPU usage (%)")
axes[0].set_ylim(0, 110)
axes[0].legend(loc="upper right", fontsize=9)
axes[0].grid(True, alpha=0.3)

# Workers active out of 8
axes[1].plot(elapsed_s, workers, linewidth=0.9, color="tab:green")
axes[1].axhline(8, color="red", linestyle="--", linewidth=0.8, label="pool size (8)")
axes[1].axhline(avg_workers, color="gray", linestyle=":", linewidth=0.8,
                label=f"avg ({avg_workers:.2f})")
axes[1].set_ylabel("workers active")
axes[1].set_ylim(0, 9)
axes[1].legend(loc="upper right", fontsize=9)
axes[1].grid(True, alpha=0.3)

# Queue length
axes[2].plot(elapsed_s, qlen, linewidth=0.9, color="tab:orange")
axes[2].axhline(peak_q, color="gray", linestyle=":", linewidth=0.8,
                label=f"peak ({peak_q})")
axes[2].set_ylabel("queue length")
axes[2].set_xlabel("elapsed time (s)")
axes[2].legend(loc="upper right", fontsize=9)
axes[2].grid(True, alpha=0.3)

fig.suptitle(f"FIFO dispatcher — {csv_path.name}  (runtime {runtime:.1f}s)",
             fontsize=12)
fig.tight_layout()
fig.savefig(out_path, dpi=140)
print(f"wrote {out_path}  ({len(elapsed_s)} samples)")