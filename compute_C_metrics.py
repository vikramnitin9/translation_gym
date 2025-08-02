#!/usr/bin/env python3
import os
import re
import sys
import subprocess
from pathlib import Path
import prettytable

# Configuration
BASE_DIR = Path(__file__).resolve().parent
COREUTILS_DIR = BASE_DIR / "data" / "coreutils" / "src_sep"
C_METRICS_BIN = BASE_DIR / "tools" / "C_metrics" / "build" / "C_metrics"


def run(cmd, cwd=None, timeout=300):
    """Run a shell command and return its stdout, raising on non-zero exit."""
    proc = subprocess.run(
        cmd,
        shell=True,
        cwd=cwd,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        timeout=timeout,
    )
    if proc.returncode != 0:
        raise RuntimeError(f"Command failed ({proc.returncode}) in {cwd or os.getcwd()}: {proc.stderr.strip()}")
    return proc.stdout


def parse_metrics_output(metrics_output):
    """
    Parse C_metrics output into a dict of counts.
    Supports both old and new output formats (uppercase labels or camelCase).
    """
    metrics = {
        'unsafe_lines': 0,
        'unsafe_calls': 0,
        'unsafe_casts': 0,
        'raw_pointer_derefs': 0,
        'raw_pointer_decls': 0,
    }
    for line in metrics_output.splitlines():
        line = line.strip()
        # camelCase format from C_metrics binary
        if m := re.match(r'unsafeLines:\s*(\d+)', line):
            metrics['unsafe_lines'] += int(m.group(1))
        elif m := re.match(r'calls:\s*(\d+)', line):
            metrics['unsafe_calls'] += int(m.group(1))
        elif m := re.match(r'casts:\s*(\d+)', line):
            metrics['unsafe_casts'] += int(m.group(1))
        elif m := re.match(r'pointerDerefs:\s*(\d+)', line):
            metrics['raw_pointer_derefs'] += int(m.group(1))
        elif m := re.match(r'pointerDecls:\s*(\d+)', line):
            metrics['raw_pointer_decls'] += int(m.group(1))
        # legacy uppercase labels (if any)
        elif m := re.match(r'Unsafe lines:\s*(\d+)', line):
            metrics['unsafe_lines'] += int(m.group(1))
        elif m := re.match(r'Unsafe calls:\s*(\d+)', line):
            metrics['unsafe_calls'] += int(m.group(1))
        elif m := re.match(r'Unsafe casts:\s*(\d+)', line):
            metrics['unsafe_casts'] += int(m.group(1))
        elif m := re.match(r'Raw pointer dereferences:\s*(\d+)', line):
            metrics['raw_pointer_derefs'] += int(m.group(1))
        elif m := re.match(r'Raw pointer declarations:\s*(\d+)', line):
            metrics['raw_pointer_decls'] += int(m.group(1))
    return metrics


def main():
    if not COREUTILS_DIR.is_dir():
        sys.exit(f"Coreutils directory not found: {COREUTILS_DIR}")

    all_metrics = {}

    for util in sorted(COREUTILS_DIR.iterdir()):
        if not util.is_dir():
            continue
        name = util.name
        # inside each util folder, code is under 'c/' or at top
        code_dir = util / 'c' if (util / 'c').is_dir() else util
        covered_fn = util / 'covered_funcs.txt'
        print(f"\n➡️ Processing coreutil: {name}")

        # 1) Generate compile_commands.json via Bear
        print("🔨 Generating compile_commands.json with Bear...")
        compdb = code_dir / 'compile_commands.json'
        if compdb.exists():
            compdb.unlink()
        run('bear --cdb compile_commands.json make', cwd=code_dir)

        # 2) Run C_metrics binary on each .c file
        metrics_output = ''
        for c_file in sorted(code_dir.glob('*.c')):
            print(f"--- metrics for {c_file.name} ---")
            cmd = f"{C_METRICS_BIN}"
            if covered_fn.exists():
                cmd += f" --covered-functions {covered_fn}"
            cmd += f" {c_file.name}"
            out = run(cmd, cwd=code_dir)
            metrics_output += out + '\n'

        # parse and store
        all_metrics[name] = parse_metrics_output(metrics_output)

    # Print summary table
    tbl = prettytable.PrettyTable()
    cols = ['unsafe_lines', 'unsafe_calls', 'unsafe_casts',
            'raw_pointer_derefs', 'raw_pointer_decls']
    tbl.field_names = ['Coreutil'] + cols
    for util, metrics in all_metrics.items():
        tbl.add_row([util] + [metrics[c] for c in cols])

    print("\n📋 Summary of all coreutils metrics:")
    print(tbl)


if __name__ == '__main__':
    main()
