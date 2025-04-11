#!/bin/bash

EXECUTABLE="toy"
EXPECTED_OUTPUT="Sum: 8
Difference: 2
Concatenation: Hello, world!"

# Run the executable and capture its output
OUTPUT=$($EXECUTABLE)

# Check if the output matches the expected output
if [[ "$OUTPUT" == "$EXPECTED_OUTPUT" ]]; then
    echo "Success: Output matches expected string."
    exit 0
else
    echo "Failure: Output does not match expected string."
    echo "Expected: '$EXPECTED_OUTPUT'"
    echo "Got: '$OUTPUT'"
    exit 1
fi
