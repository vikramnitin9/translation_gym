from pathlib import Path
import os
import subprocess
import re
import json
import prettytable

def run(command, timeout=120):

    try:
        result = subprocess.run(
            command,
            shell=True,
            timeout=timeout,
            stderr=subprocess.PIPE,
            stdout=subprocess.PIPE,
        )
        if result.returncode != 0:
            stderr = result.stderr.decode('utf-8', errors='ignore') if result.stderr else ''
            stdout = result.stdout.decode('utf-8', errors='ignore') if result.stdout else ''
            raise RuntimeError(f"STDOUT:\n{stdout}\nSTDERR:\n{stderr}")
    except subprocess.TimeoutExpired:
        raise RuntimeError("Timeout")
    except subprocess.CalledProcessError as e:
        raise RuntimeError(e.output.decode('utf-8', errors='ignore'))
    except Exception as e:
        raise RuntimeError(str(e))

    return result.stdout.decode('utf-8', errors='ignore')

def parse_metrics_output(metrics_output):
    metrics = {}
    for line in metrics_output.split('\n'):
        
        # If the line is of the format "Unsafe spans: <num>", extract the num
        unsafe_spans = re.match(r'Unsafe lines: (\d+)', line)
        if unsafe_spans:
            if 'unsafe_spans' in metrics:
                metrics['unsafe_spans'] += int(unsafe_spans.group(1))
            else:
                metrics['unsafe_spans'] = int(unsafe_spans.group(1))

        unsafe_calls = re.match(r'Unsafe calls: (\d+)', line)
        if unsafe_calls:
            if 'unsafe_calls' in metrics:
                metrics['unsafe_calls'] += int(unsafe_calls.group(1))
            else:
                metrics['unsafe_calls'] = int(unsafe_calls.group(1))

        unsafe_casts = re.match(r'Unsafe casts: (\d+)', line)
        if unsafe_casts:
            if 'unsafe_casts' in metrics:
                metrics['unsafe_casts'] += int(unsafe_casts.group(1))
            else:
                metrics['unsafe_casts'] = int(unsafe_casts.group(1))
        
        # If the line is of the format "Raw pointer dereferences: <num>", extract the num
        raw_pointer_derefs = re.match(r'Raw pointer dereferences: (\d+)', line)
        if raw_pointer_derefs:
            if 'raw_pointer_derefs' in metrics:
                metrics['raw_pointer_derefs'] += int(raw_pointer_derefs.group(1))
            else:
                metrics['raw_pointer_derefs'] = int(raw_pointer_derefs.group(1))
        
        # If the line is of the format "Raw pointer declarations: <num>", extract the num
        raw_pointer_decls = re.match(r'Raw pointer declarations: (\d+)', line)
        if raw_pointer_decls:
            if 'raw_pointer_decls' in metrics:
                metrics['raw_pointer_decls'] += int(raw_pointer_decls.group(1))
            else:
                metrics['raw_pointer_decls'] = int(raw_pointer_decls.group(1))
        
    return metrics

if __name__ == "__main__":

    all_metrics = {}
    root_dir = Path('.').resolve()

    FILTER = False
    if FILTER:
        filter_arg = ' -- -Zselected-fns-file="../covered_funcs.txt"'
    else:
        filter_arg = ''

    # Go to each directory in output/{}
    for dir in Path('output').glob('*'):
        if not dir.is_file():
            dir = dir.name
            print(f"Processing {dir}...")
            os.chdir(root_dir)
            subdir = Path('output/{}'.format(dir))
            os.chdir(subdir)
            try:
                cmd = f'C_BUILD_PATH="$(pwd)/source" RUSTFLAGS="-Awarnings" cargo metrics{filter_arg}'
                metrics_output = run(cmd)
            except RuntimeError as e:
                print(f"Error in {subdir}: \n{e}")
                continue
            os.chdir(root_dir)

            all_metrics[dir] = parse_metrics_output(metrics_output)
            
    # Write this dict to json
    with open('metrics.json', 'w') as f:
        json.dump(all_metrics, f, indent=4)

    # Print the metrics in a table format
    table = prettytable.PrettyTable()
    value = next(iter(all_metrics.values()))
    keys = list(value.keys())
    table.field_names = ["Folder"] + keys
    for dir, metrics in all_metrics.items():
        table.add_row([dir] + [metrics[key] for key in keys])

    print(table)