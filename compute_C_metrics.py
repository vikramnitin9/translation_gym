#!/usr/bin/env python3
import os, re, sys, subprocess, csv
from pathlib import Path
import prettytable

BASE_DIR = Path(__file__).resolve().parent
COREUTILS_DIR = BASE_DIR / "data" / "coreutils" / "src_sep"
C_METRICS_BIN = Path("/usr/local/bin/c_metrics")
print(f"Using C_metrics: {C_METRICS_BIN}")
OUT_DIR = BASE_DIR / "output"  # NEW

def run(cmd, cwd=None, timeout=300):
    proc = subprocess.run(
        cmd, shell=True, cwd=cwd,
        stdout=subprocess.PIPE, stderr=subprocess.PIPE,
        text=True, timeout=timeout,
    )
    if proc.returncode != 0:
        raise RuntimeError(
            f"Command failed ({proc.returncode}) in {cwd or os.getcwd()}:\n{proc.stderr.strip()}"
        )
    return proc.stdout

def parse_metrics_output(metrics_output: str):
    metrics = {
        'unsafe_lines': 0,
        'unsafe_calls': 0,
        'unsafe_casts': 0,
        'raw_pointer_derefs': 0,
        'raw_pointer_decls': 0,
    }
    for line in metrics_output.splitlines():
        s = line.strip()
        m = re.match(r'unsafeLines:\s*(\d+)', s)
        if m: metrics['unsafe_lines'] += int(m.group(1)); continue
        m = re.match(r'calls:\s*(\d+)', s)
        if m: metrics['unsafe_calls'] += int(m.group(1)); continue
        m = re.match(r'casts:\s*(\d+)', s)
        if m: metrics['unsafe_casts'] += int(m.group(1)); continue
        m = re.match(r'pointerDerefs:\s*(\d+)', s)
        if m: metrics['raw_pointer_derefs'] += int(m.group(1)); continue
        m = re.match(r'pointerDecls:\s*(\d+)', s)
        if m: metrics['raw_pointer_decls'] += int(m.group(1)); continue
        # legacy labels
        m = re.match(r'Unsafe lines:\s*(\d+)', s)
        if m: metrics['unsafe_lines'] += int(m.group(1)); continue
        m = re.match(r'Unsafe calls:\s*(\d+)', s)
        if m: metrics['unsafe_calls'] += int(m.group(1)); continue
        m = re.match(r'Unsafe casts:\s*(\d+)', s)
        if m: metrics['unsafe_casts'] += int(m.group(1)); continue
        m = re.match(r'Raw pointer dereferences:\s*(\d+)', s)
        if m: metrics['raw_pointer_derefs'] += int(m.group(1)); continue
        m = re.match(r'Raw pointer declarations:\s*(\d+)', s)
        if m: metrics['raw_pointer_decls'] += int(m.group(1)); continue
    return metrics

def main():
    if not COREUTILS_DIR.is_dir():
        sys.exit(f"Coreutils directory not found: {COREUTILS_DIR}")
    if not C_METRICS_BIN.is_file():
        sys.exit(f"C_metrics binary not found: {C_METRICS_BIN}")

    OUT_DIR.mkdir(parents=True, exist_ok=True)  # NEW

    all_metrics = {}
    cols = ['unsafe_lines','unsafe_calls','unsafe_casts','raw_pointer_derefs','raw_pointer_decls']

    # iterate all utils
    for util_dir in sorted(p for p in COREUTILS_DIR.iterdir() if p.is_dir()):
        util = util_dir.name
        code_dir = util_dir / "c"
        if not code_dir.is_dir():
            print(f"⚠️  Skipping {util}: {code_dir} not found")
            continue

        covered_fn = util_dir / "covered_funcs.txt"
        print(f"\n➡️ Processing coreutil: {util}")

        metrics_output = ''
        for c_file in sorted(code_dir.glob("*.c")):
            print(f"--- metrics for {c_file.name} ---")
            cmd = f"{C_METRICS_BIN}"
            if covered_fn.exists():
                cmd += f" --covered-functions {covered_fn}"
            cmd += f" {c_file.name}"
            out = run(cmd, cwd=code_dir)
            metrics_output += out + "\n"

        all_metrics[util] = parse_metrics_output(metrics_output)

    # Pretty print summary
    tbl = prettytable.PrettyTable()
    tbl.field_names = ["Coreutil"] + cols
    for util in sorted(all_metrics.keys()):
        tbl.add_row([util] + [all_metrics[util][c] for c in cols])
    print("\n📋 Summary of all coreutils metrics:")
    print(tbl)

    # NEW: write CSV summary
    summary_csv = OUT_DIR / "coreutils_metrics_summary.csv"
    with summary_csv.open("w", newline="") as f:
        w = csv.writer(f)
        w.writerow(["Coreutil"] + cols)
        for util in sorted(all_metrics.keys()):
            w.writerow([util] + [all_metrics[util][c] for c in cols])
    print(f"\n📝 Wrote summary CSV -> {summary_csv}")

if __name__ == "__main__":
    main()
