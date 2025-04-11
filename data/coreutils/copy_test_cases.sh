#!/bin/bash
SRC_DIR="./src"
TEST_DIR="./tests"
COREUTILS_DIR="/tmp/coreutils"

rm -rf "$TEST_DIR"
mkdir -p "$TEST_DIR"

cp "$COREUTILS_DIR/tests/init.sh" "$TEST_DIR"

for dir in $SRC_DIR/*; do
    prog_name="${dir##*/}"
    mkdir -p "$TEST_DIR/$prog_name"
    cp $COREUTILS_DIR/tests/$prog_name/* "$TEST_DIR/$prog_name/"
done

find "$TEST_DIR" -mindepth 2 -maxdepth 2 -type f -name "*.sh" -print0 | while read -d '' file; do
    sed -i "s@path_prepend_ ./src@path_prepend_ \$1@g" "$file"
    sed -i "/print_ver_ .*/d" "$file"
done
