#!/usr/bin/env python3
"""
Plot the monitor CSV produced by optimized_scheduler_fixed.rs.

Default Rust CSV header:
    elapsed_ms,cpu_budget_used,workers_active,cpu_running,queue_io,queue_cpu,queue_len

Usage:
    python plot_monitor_optimized_fixed.py
    python plot_monitor_optimized_fixed.py monitor_log_optimized_fixed.csv
    python plot_monitor_optimized_fixed.py monitor_log_optimized_fixed.csv optimized_plot.png
    python plot_monitor_optimized_fixed.py --cpu-cap 100 --workers 8

Output:
    A PNG with three stacked panels:
      1) simulated CPU budget used
      2) active workers and CPU-kind workers
      3) queue length, split into IO and CPU queues when available

Notes:
    - The CPU value is the scheduler's simulated CPU budget, not OS CPU usage.
    - If config.toml is present, this script tries to read cpu_cap_pct and num_workers.
    - Command-line --cpu-cap and --workers override config.toml.
"""

from __future__ import annotations

import argparse
import csv
import sys
from pathlib import Path
from typing import Dict, List, Optional

import matplotlib

matplotlib.use("Agg")  # headless backend: works in Codespaces/SSH
import matplotlib.pyplot as plt


DEFAULT_CSV = "monitor_log_optimized_fixed.csv"
DEFAULT_OUT = "monitor_plot_optimized_fixed.png"


def read_simple_config(path: Path) -> Dict[str, str]:
    """Read the simple key=value config.toml style used by the Rust file."""
    values: Dict[str, str] = {}
    if not path.exists():
        return values

    for raw in path.read_text().splitlines():
        line = raw.split("#", 1)[0].strip()
        if not line or "=" not in line:
            continue
        key, value = line.split("=", 1)
        values[key.strip()] = value.strip().strip('"')
    return values


def get_int(row: Dict[str, str], names: List[str], default: Optional[int] = None) -> int:
    """Read the first available integer column from a CSV row."""
    for name in names:
        if name in row and row[name] != "":
            return int(row[name])
    if default is not None:
        return default
    raise KeyError(f"missing required column; tried {names}")


def parse_args() -> argparse.Namespace:
    config = read_simple_config(Path("config.toml"))

    config_csv = config.get("csv_path", DEFAULT_CSV)
    config_cpu_cap = int(config.get("cpu_cap_pct", "100"))
    config_workers = int(config.get("num_workers", "8"))

    parser = argparse.ArgumentParser(
        description="Plot monitor CSV from optimized_scheduler_fixed.rs"
    )
    parser.add_argument(
        "csv_path",
        nargs="?",
        default=config_csv,
        help=f"input CSV path, default: {config_csv}",
    )
    parser.add_argument(
        "out_path",
        nargs="?",
        default=DEFAULT_OUT,
        help=f"output PNG path, default: {DEFAULT_OUT}",
    )
    parser.add_argument(
        "--cpu-cap",
        type=int,
        default=config_cpu_cap,
        help=f"simulated CPU budget cap line, default: {config_cpu_cap}",
    )
    parser.add_argument(
        "--workers",
        type=int,
        default=config_workers,
        help=f"worker pool size cap line, default: {config_workers}",
    )
    parser.add_argument(
        "--title",
        default="Optimized scheduler, fixed",
        help="title prefix for the plot",
    )
    return parser.parse_args()


def main() -> None:
    args = parse_args()
    csv_path = Path(args.csv_path)
    out_path = Path(args.out_path)

    if not csv_path.exists():
        sys.exit(f"error: {csv_path} not found. Run the Rust simulation first.")

    elapsed_s: List[float] = []
    cpu_budget: List[int] = []
    workers_active: List[int] = []
    cpu_running: List[int] = []
    queue_io: List[int] = []
    queue_cpu: List[int] = []
    queue_len: List[int] = []

    with csv_path.open(newline="") as f:
        reader = csv.DictReader(f)
        if reader.fieldnames is None:
            sys.exit("error: CSV has no header row.")

        for row in reader:
            elapsed_ms = get_int(row, ["elapsed_ms"])
            elapsed_s.append(elapsed_ms / 1000.0)

            # New fixed scheduler column is cpu_budget_used.
            # cpu_pct is accepted only as a fallback for older FIFO logs.
            cpu_budget.append(get_int(row, ["cpu_budget_used", "cpu_pct"]))
            workers_active.append(get_int(row, ["workers_active"]))
            cpu_running.append(get_int(row, ["cpu_running"], default=0))

            io_q = get_int(row, ["queue_io"], default=0)
            cpu_q = get_int(row, ["queue_cpu"], default=0)
            queue_io.append(io_q)
            queue_cpu.append(cpu_q)

            if "queue_len" in row and row["queue_len"] != "":
                queue_len.append(int(row["queue_len"]))
            else:
                queue_len.append(io_q + cpu_q)

    if not elapsed_s:
        sys.exit("error: CSV has no data rows.")

    runtime = elapsed_s[-1]
    avg_cpu = sum(cpu_budget) / len(cpu_budget)
    avg_workers = sum(workers_active) / len(workers_active)
    peak_cpu = max(cpu_budget)
    peak_workers = max(workers_active)
    peak_q = max(queue_len)
    peak_io_q = max(queue_io) if queue_io else 0
    peak_cpu_q = max(queue_cpu) if queue_cpu else 0

    fig, axes = plt.subplots(3, 1, figsize=(11, 8), sharex=True)

    # Panel 1: simulated CPU budget.
    axes[0].plot(elapsed_s, cpu_budget, linewidth=0.9, label="used")
    axes[0].axhline(args.cpu_cap, linestyle="--", linewidth=0.8, label=f"cap ({args.cpu_cap}%)")
    axes[0].axhline(avg_cpu, linestyle=":", linewidth=0.8, label=f"avg ({avg_cpu:.1f}%)")
    axes[0].set_ylabel("CPU budget (%)")
    axes[0].set_ylim(0, max(args.cpu_cap, peak_cpu) * 1.10 + 1)
    axes[0].legend(loc="upper right", fontsize=9)
    axes[0].grid(True, alpha=0.3)

    # Panel 2: workers.
    axes[1].plot(elapsed_s, workers_active, linewidth=0.9, label="workers active")
    axes[1].plot(elapsed_s, cpu_running, linewidth=0.9, label="CPU-kind workers")
    axes[1].axhline(args.workers, linestyle="--", linewidth=0.8, label=f"pool size ({args.workers})")
    axes[1].axhline(avg_workers, linestyle=":", linewidth=0.8, label=f"avg active ({avg_workers:.2f})")
    axes[1].set_ylabel("workers")
    axes[1].set_ylim(0, max(args.workers, peak_workers) + 1)
    axes[1].legend(loc="upper right", fontsize=9)
    axes[1].grid(True, alpha=0.3)

    # Panel 3: queue split.
    axes[2].plot(elapsed_s, queue_len, linewidth=0.9, label="total queue")
    if any(queue_io) or any(queue_cpu):
        axes[2].plot(elapsed_s, queue_io, linewidth=0.8, label="IO queue")
        axes[2].plot(elapsed_s, queue_cpu, linewidth=0.8, label="CPU queue")
    axes[2].axhline(peak_q, linestyle=":", linewidth=0.8, label=f"peak total ({peak_q})")
    axes[2].set_ylabel("queued tasks")
    axes[2].set_xlabel("elapsed time (s)")
    axes[2].set_ylim(0, peak_q * 1.10 + 1)
    axes[2].legend(loc="upper right", fontsize=9)
    axes[2].grid(True, alpha=0.3)

    fig.suptitle(
        f"{args.title} — {csv_path.name}  (runtime {runtime:.1f}s)",
        fontsize=12,
    )
    fig.tight_layout()
    fig.savefig(out_path, dpi=140)

    print(f"wrote {out_path} ({len(elapsed_s)} samples)")
    print(
        "summary: "
        f"avg_cpu={avg_cpu:.2f}%, peak_cpu={peak_cpu}%, "
        f"avg_workers={avg_workers:.2f}, peak_workers={peak_workers}, "
        f"peak_queue={peak_q} (IO={peak_io_q}, CPU={peak_cpu_q})"
    )


if __name__ == "__main__":
    main()
