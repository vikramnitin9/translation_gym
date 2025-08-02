#!/usr/bin/env python3
import os
import re
import sys
import subprocess
from pathlib import Path
import prettytable

# -----------------------------------------------------
# compute_C_metrics.py
# -----------------------------------------------------
# Iterate over each instrumented output under 'output/',
# copy the original compile_commands.json from data/coreutils/src_sep,
# run C_metrics on the instrumented C source in 'source/',
# using the covered_funcs.txt, and summarize results.
# -----------------------------------------------------

# Paths
BASE_DIR = Path(__file__).resolve().parent
OUTPUT_DIR = BASE_DIR / 'output'
COREUTILS_SRC = BASE_DIR / 'data' / 'coreutils' / 'src_sep'
C_METRICS_BIN = BASE_DIR / 'tools' / 'C_metrics' / 'build' / 'C_metrics'


def run(cmd, cwd=None, timeout=300):
    proc = subprocess.run(
        cmd, shell=True, cwd=cwd,
        stdout=subprocess.PIPE, stderr=subprocess.PIPE,
        text=True, timeout=timeout
    )
    if proc.returncode != 0:
        raise RuntimeError(f"Command '{cmd}' failed in {cwd or os.getcwd()}: {proc.stderr.strip()}")
    return proc.stdout


def parse_metrics_output(output: str):
    stats = {k: 0 for k in ['pointerDecls','pointerDerefs','casts','calls','unsafeLines']}
    for line in output.splitlines():
        line = line.strip()
        for key in stats:
            if line.startswith(f"{key}:"):
                try:
                    stats[key] += int(line.split(':',1)[1].strip())
                except ValueError:
                    pass
    return stats


def main():
    if not OUTPUT_DIR.is_dir():
        sys.exit(f"Output directory not found: {OUTPUT_DIR}")

    results = {}

    for out_sub in sorted(OUTPUT_DIR.iterdir()):
        if not out_sub.is_dir():
            continue
        util = out_sub.name.split('_')[0]
        instr_src = out_sub / 'source'
        if not instr_src.is_dir():
            print(f"Skipping {out_sub.name}: no source/ dir")
            continue

        # Find original compile_commands.json
        orig_c_dir = COREUTILS_SRC / util / 'c'
        if not orig_c_dir.exists():
            orig_c_dir = COREUTILS_SRC / util
        orig_db = orig_c_dir / 'compile_commands.json'
        if not orig_db.exists():
            print(f"Warning: original compile_commands.json not found for {util}, skipping.")
            continue

        # Copy compile_commands.json into instrumented source dir
        dst_db = instr_src / 'compile_commands.json'
        print(f"\n➡️  {util}: copying compile_commands.json → {dst_db}")
        run(f"cp {orig_db} {dst_db}")

        # Covered functions file
        cov1 = out_sub / 'covered_funcs.txt'
        cov2 = out_sub / 'covered_functions.txt'
        covered = cov1 if cov1.exists() else (cov2 if cov2.exists() else None)
        cov_arg = f"--covered-functions {covered.name}" if covered else ''
        if covered:
            # copy it into source dir
            run(f"cp {covered} {instr_src}/{covered.name}")

        # Run metrics on each .c
        agg = {k:0 for k in ['pointerDecls','pointerDerefs','casts','calls','unsafeLines']}
        print(f"Processing instrumented util: {out_sub.name}")
        for src_file in sorted(instr_src.glob('*.c')):
            print(f"--- metrics for {src_file.name} ---")
            cmd = f"{C_METRICS_BIN} {cov_arg} {src_file.name}".strip()
            out = run(cmd, cwd=instr_src)
            stats = parse_metrics_output(out)
            for k,v in stats.items(): agg[k] += v

        results[util] = agg

    # Print summary
    tbl = prettytable.PrettyTable()
    cols = ['pointerDecls','pointerDerefs','casts','calls','unsafeLines']
    tbl.field_names = ['Coreutil'] + cols
    for util, stats in sorted(results.items()):
        tbl.add_row([util] + [stats[c] for c in cols])

    print("\n📋 Summary of all instrumented coreutils metrics:")
    print(tbl)

if __name__=='__main__':
    main()
