import json
import subprocess

with open('/app/input_output.json', 'r') as f:
    data = json.load(f)

for item in data:
    input_num = item['input']
    expected_output = item['output']
    
    # Run the URL parser script
    result = subprocess.run(['urlparser-test', str(input_num)], stdout=subprocess.PIPE, text=True)

    # Check if the output matches the expected output
    if result.stdout.strip() == expected_output:
        print(f"Test passed for input: {input_num}")
    else:
        print(f"Test failed for input: {input_num}\nExpected:\n{expected_output}\nActual:\n{result.stdout.strip()}")
        exit(1)