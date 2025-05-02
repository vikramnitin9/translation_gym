#!/bin/bash

# Define the root directory (default to current directory if not provided)
ROOT_DIR="${1:-.}"

# Define the original line and the replacement line
OLD_LINE='. "${srcdir=.}/tests/init.sh"; path_prepend_ ./src'
NEW_LINE='. "${srcdir=.}/tests/init.sh"; path_prepend_ ./src; path_prepend_ /executable'

# Find all .sh files and process them
find "$ROOT_DIR" -type f -name "*.sh" | while read -r file; do
  if grep -Fq "$OLD_LINE" "$file"; then
    sed -i.bak "s|$OLD_LINE|$NEW_LINE|" "$file" && rm "${file}.bak"
    echo "Updated: $file"
  fi
done
