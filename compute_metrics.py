from pathlib import Path
import os
import subprocess
import re
import json
import prettytable
import functools

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

def parse_c_metrics_output(metrics_output):
    metrics = {}
    for line in metrics_output.split('\n'):
        
        # If the line is of the format "Unsafe spans: <num>", extract the num
        unsafe_lines = re.match(r'unsafeLines: (\d+)', line)
        if unsafe_lines:
            metrics['c_lines'] = int(unsafe_lines.group(1))

        c_calls = re.match(r'calls: (\d+)', line)
        if c_calls:
            metrics['c_calls'] = int(c_calls.group(1))

        c_casts = re.match(r'casts: (\d+)', line)
        if c_casts:
            metrics['c_casts'] = int(c_casts.group(1))

        # If the line is of the format "Raw pointer dereferences: <num>", extract the num
        c_pointer_derefs = re.match(r'pointerDerefs: (\d+)', line)
        if c_pointer_derefs:
            metrics['c_pointer_derefs'] = int(c_pointer_derefs.group(1))

        # If the line is of the format "Raw pointer declarations: <num>", extract the num
        c_pointer_decls = re.match(r'pointerDecls: (\d+)', line)
        if c_pointer_decls:
            metrics['c_pointer_decls'] = int(c_pointer_decls.group(1))

    return metrics

def parse_metrics_output(metrics_output):
    metrics = {}
    for line in metrics_output.split('\n'):
        
        # If the line is of the format "Unsafe spans: <num>", extract the num
        unsafe_lines = re.match(r'Unsafe lines: (\d+)', line)
        if unsafe_lines:
            if 'unsafe_lines' in metrics:
                metrics['unsafe_lines'] += int(unsafe_lines.group(1))
            else:
                metrics['unsafe_lines'] = int(unsafe_lines.group(1))

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

def run_c_metrics_for_path(c_code_path, covered_funcs_path=None):
    """
    Run c_metrics for the given C code path and covered functions path.
    """
    cwd = os.getcwd()
    if not c_code_path.exists():
        print(f"Error: {c_code_path} does not exist")
        return None
    # Delete compile_commands.json if it exists
    compile_commands_json = c_code_path / "compile_commands.json"
    if compile_commands_json.exists():
        compile_commands_json.unlink()
    os.chdir(c_code_path)
    try:
        bear_version = run("bear --version")
    except RuntimeError as e:
        print(f"Error: Unable to get bear version. {e}")
        os.chdir(cwd)
        return None
    version = bear_version.split()[1]
    major, _, _ = version.split('.')
    if int(major) >= 3:
        cmd = 'make clean && bear -- make'
    else:
        cmd = "make clean && bear make"
    try:
        run(cmd)
    except RuntimeError as e:
        print(f"Error: Unable to run bear. {e}")
    finally:
        os.chdir(cwd)
    
    if not compile_commands_json.exists():
        print(f"Error: {compile_commands_json} does not exist after running bear. Something wrong.")
        os.chdir(cwd)
        return None

    if covered_funcs_path and not covered_funcs_path.exists():
        print(f"Error: {covered_funcs_path} does not exist")
        os.chdir(cwd)
        return None
    
    cwd = os.getcwd()
    # Check if C_METRICS_BUILD_DIR is set
    c_metrics_build_dir = os.environ.get('C_METRICS_BUILD_DIR')
    if c_metrics_build_dir is None:
        print("Error: $C_METRICS_BUILD_DIR not set.")
        os.chdir(cwd)
        return None
    try:
        os.chdir(c_code_path)
        covered_funcs_arg = f"--covered-functions {covered_funcs_path} " if covered_funcs_path else ""
        c_metrics_output = run(f'{c_metrics_build_dir}/c_metrics {covered_funcs_arg}*.c')
    except RuntimeError as e:
        print(f"Error running c_metrics: {e}")
        os.chdir(cwd)
        return None
    finally:
        print(f"Ran c_metrics in {c_code_path}")
        os.chdir(cwd)
    
    return parse_c_metrics_output(c_metrics_output)

if __name__ == "__main__":

    all_metrics = {}
    root_dir = Path('.').resolve()

    FILTER = False
    if FILTER:
        filter_arg = ' -- -Zselected-fns-file="../covered_funcs.txt"'
    else:
        filter_arg = ''
    
    dataset_json_path = Path('data/datasets.json')
    if not dataset_json_path.exists():
        print(f"Error: {dataset_json_path} does not exist")
        exit(1)
    dataset_json = json.loads(dataset_json_path.read_text())

    # Go to each directory in output/{}
    for dir in Path('output').glob('*'):
        if not dir.is_file():
            dir = dir.name
            print(f"Processing {dir}...")
            dataset_name = dir.split('_')[0]
            if dataset_name not in dataset_json:
                print(f"Error: {dataset_name} not found in datasets.json")
                continue
            os.chdir(root_dir)
            if dataset_name not in all_metrics:
                print("Running C metrics in the original directory to get 'before' metrics...")
                c_code_path = root_dir / "data" / dataset_json[dataset_name]['code_dir']
                if dataset_json[dataset_name]['covered_fn_list'] is None:
                    covered_funcs_path = None
                else:
                    covered_funcs_path = (root_dir / "data" / dataset_json[dataset_name]['covered_fn_list']).absolute()
                metrics_output = run_c_metrics_for_path(c_code_path, covered_funcs_path)
                if metrics_output is None:
                    print(f"Error running C metrics in {c_code_path}")
                else:
                    all_metrics[dataset_name] = metrics_output

            subdir = Path('output/{}'.format(dir)).absolute()
            os.chdir(subdir)
            c_build_path = subdir / "source"

            if dataset_json[dataset_name]['covered_fn_list'] is None:
                covered_funcs_path = None
            else:
                covered_funcs_path = (root_dir / "data" / dataset_json[dataset_name]['covered_fn_list']).absolute()
            metrics_output = run_c_metrics_for_path(c_build_path, covered_funcs_path)
            if metrics_output is None:
                print(f"Error running C metrics in {c_build_path}")
                continue
            all_metrics[dir] = metrics_output
            
            try:
                cmd = f'C_BUILD_PATH="$(pwd)/source" RUSTFLAGS="-Awarnings" cargo metrics{filter_arg}'
                metrics_output = run(cmd)
            except RuntimeError as e:
                print(f"Error running cargo metrics for {subdir}: \n{e}")
                continue
            finally:
                print(f"Ran Rust metrics in {subdir}")
                os.chdir(root_dir)

            all_metrics[dir].update(parse_metrics_output(metrics_output))
            
    # Write this dict to json
    with open((root_dir / "output" / "metrics.json"), 'w') as f:
        json.dump(all_metrics, f, indent=4)
    
    # Print the metrics in a table format
    table = prettytable.PrettyTable()
    value = functools.reduce(lambda a, b: a | b, [set(values.keys()) for values in all_metrics.values()])
    keys = sorted(list(value))
    table.field_names = ["Folder"] + keys
    for dir, metrics in all_metrics.items():
        table.add_row([dir] + [metrics.get(key, "-") for key in keys])

    print(table)