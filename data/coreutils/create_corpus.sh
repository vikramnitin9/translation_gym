#!/bin/bash

PROGRAMS_TO_TRANSLATE=(
    "sleep"
    "yes"
    "uniq"
    "dirname"
    "rmdir"
    "users"
    "kill"
    "wc"
    "printf"
    "cut"
    "head"
    "touch"
    "true"
    "date"
    "truncate"
    "cp"
    "paste"
    "split"
    "cat"
    "who"
    "tail"
    "pwd"
    "join"
    "factor"
    "false"
    "whoami"
    "echo"
    "sort"
)

OUT_DIR=/tmp/c2rust_corpus/coreutils

mkdir -p "$OUT_DIR"

for prog in "${PROGRAMS_TO_TRANSLATE[@]}"; do
    echo "Working on '$prog' in the background..."
    python3 create_c_and_rust_versions.py \
        --program_name "$prog" \
        --coreutils_dir /tmp/coreutils/ \
        > "$OUT_DIR"/"$prog".log 2>&1
done
