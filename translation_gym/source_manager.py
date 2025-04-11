from translation_gym.helpers import *

class SourceManager:

    def __init__(self, code_dir, test_dir):
        self.code_dir = code_dir
        self.test_dir = test_dir
        self.test_paths = None
        self.c_code_dir = Path(self.code_dir)/'c_src'
        self.cargo_bin_target = 'foo' # Placeholder
        self.bindgen_blocklist = Path(self.code_dir, 'bindgen_blocklist.txt')
        if not self.bindgen_blocklist.exists():
            self.bindgen_blocklist.touch()
    
    def get_bin_target(self):

        cwd = os.getcwd()
        os.chdir(self.c_code_dir)

        # Get the list of executable targets in the Makefile
        command = """make -pq | awk -F' ' '
        /^[a-zA-Z0-9_-]+:([^=]|$)/ {
            target=$1;
            gsub(/:/, "", target);
            if (target !~ /^\\./) targets[target]=1
        }
        /^\\.PHONY:/ { for (i=2; i<=NF; i++) { phony[$i]=1 } }
        END { for (t in targets) {
            if (!(t in phony) && (t != "Makefile")) { print t }
        }}'"""

        try:
            result = subprocess.run(
                command,
                shell=True,
                timeout=20,
                stderr=subprocess.PIPE,
                stdout=subprocess.PIPE,
            )
            if result.returncode != 0:
                exec_output = result.stderr.decode('utf-8', errors='ignore')
                if exec_output.strip() == '':
                    exec_output = result.stdout.decode('utf-8', errors='ignore')
                raise RunException(exec_output)
        except subprocess.TimeoutExpired:
            raise RunException("Timeout")
        
        targets = result.stdout.decode('utf-8', errors='ignore').split('\n')
        targets = [target for target in targets if target != '']
        if len(targets) == 0:
            raise Exception("No executable targets found in Makefile.")
        elif len(targets) > 1:
            raise NotImplementedError(f"Multiple executable targets ({','.join(targets)}) are not supported.")
        else:
            target = targets[0]
            prGreen(f"Found executable target: {target}")
        os.chdir(cwd)
        return target
    
    def set_cargo_bin_target(self, target):
        # Set the target in the Cargo.toml file
        cargo_toml = Path(self.code_dir, 'Cargo.toml')
        with open(cargo_toml, 'r') as f:
            lines = f.readlines()
        inside_bin = False
        for i, line in enumerate(lines):
            if line.startswith('[[bin]]'):
                inside_bin = True
            elif inside_bin and line.startswith('name ='):
                # Found the name line, replace it
                lines[i] = f'name = "{target}"\n'
                break
        with open(cargo_toml, 'w') as f:
            f.writelines(lines)
        self.cargo_bin_target = target
    
    def get_static_analysis_results(self):
        functions_json_path = Path(self.code_dir)/'c_src'/'functions.json'
        return json.load(open(functions_json_path, 'r'))
    
    def get_executable(self):
        # Check if the executable exists
        executable = Path(self.code_dir, f'target/debug/{self.cargo_bin_target}')
        if not executable.exists():
            raise Exception("Executable not found. Please compile the code first.")
        return executable
    
    def get_test_paths(self):
        if self.test_paths is not None:
            return self.test_paths
        elif self.test_dir == '':
            self.test_paths = []
            return []
        else:
            # Get a list of .sh files in test_dir
            test_files = [f for f in os.listdir(self.test_dir) if f.endswith('.sh')]
            return [Path(os.path.join(self.test_dir, test_file)) for test_file in test_files]
    
    def set_test_paths(self, test_paths):
        self.test_paths = test_paths
    
    def compile(self, verbose=False):
        cwd = os.getcwd()
        cmd = 'cd {} && RUSTFLAGS="-Awarnings" cargo build'.format(self.code_dir)

        try:
            result = subprocess.run(
                        cmd,
                        shell=True,
                        timeout=30,
                        stderr=subprocess.STDOUT if verbose else subprocess.PIPE,
                        stdout=None if verbose else subprocess.PIPE,
                    )
            if result.returncode != 0:
                if not verbose:
                    compiler_output = result.stderr.decode('utf-8', errors='ignore')
                else:
                    compiler_output = result.stdout.decode('utf-8', errors='ignore')
                raise CompileException(compiler_output)

        except (subprocess.TimeoutExpired, TimeoutError):
            raise CompileException("Timeout")

        finally:
            os.chdir(cwd)
    
    def extract_body(self, func):
        fpath = Path(os.path.join(self.c_code_dir, func['filename']))
        start_line = func['startLine']
        start_col = func['startCol']
        end_line = func['endLine']
        end_col = func['endCol']

        with open(fpath, 'r') as f:
            lines = f.readlines()
        
        body = lines[start_line-1][start_col-1:]
        for i in range(start_line, end_line-1):
            body += lines[i]
        body += lines[end_line-1][:end_col]
        return body

    def insert_translation(self, func, translation):

        self.comment_out_in_c(func)
        self.insert_in_rust(translation)
        # Add the function to the bindgen blocklist
        shutil.copy(self.bindgen_blocklist, self.bindgen_blocklist.with_suffix('.old'))
        with open(self.bindgen_blocklist, 'a') as f:
            f.write(f"{func['name']}\n")
    
    def comment_out_in_c(self, func):

        fpath = Path(os.path.join(self.c_code_dir, func['filename']))
        start_line = func['startLine']
        start_col = func['startCol']
        end_line = func['endLine']
        end_col = func['endCol']

        with open(fpath, 'r') as f:
            lines = f.readlines()

        old_lines = lines.copy()
        lines = [line.rstrip() for line in lines]

        # We can't do a multiline comment because C doesn't support nested multiline comments
        lines[start_line-1] = lines[start_line-1][:start_col-1] + '// ' + lines[start_line-1][start_col-1:]
        for i in range(start_line, end_line-1):
            lines[i] = '// ' + lines[i]
        if lines[end_line-1][end_col:] != '':
            # Taking advantage of the fact that you can insert a linebreak anywhere in C, for the most part
            lines[end_line-1] = '// ' + lines[end_line-1][:end_col] + '\n' + lines[end_line-1][end_col:]
        else:
            lines[end_line-1] = '// ' + lines[end_line-1]
        
        with open(fpath, 'w') as f:
            f.write('\n'.join(lines))
        with open(fpath.with_suffix('.old'), 'w') as f:
            f.write(''.join(old_lines))

    def insert_in_rust(self, translation):

        function_trans = translation['func']
        wrapper = translation['wrapper']
        imports = translation['imports'] if 'imports' in translation else ''

        # Write the function translation to src/main.rs

        # Read the contents of the main.rs file
        main_rs = Path(self.code_dir, 'src/main.rs')
        contents = main_rs.read_text()
        old_contents = contents
        lines = contents.split('\n')

        inside_attribute = False
        for i, line in enumerate(lines):
            # Check if line starts with "#!" (inner attribute)
            # These could also extend over multiple lines, which is tricky
            # Like #![feature(try_blocks), 
            #       feature(never_type)]
            if inside_attribute:
                if ']' in line:
                    inside_attribute = False
                continue
            if line.startswith('#!'):
                if ']' not in line:
                    inside_attribute = True
                continue
            # There can be empty lines between inner attribute lines
            if line.strip() == '':
                continue
            break
        lines.insert(i, imports)
        new_contents = '\n'.join(lines)

        # Insert the function_trans and wrapper at the bottom
        new_contents += '\n' + function_trans + '\n' + wrapper

        main_rs.write_text(new_contents)
        # De-duplicate imports
        try:
            run(f'cd {self.code_dir} && rustfmt --config imports_granularity=Crate src/main.rs')
        except:
            prRed("Rustfmt failed. There may be a syntax error in the generated code.")

        with open(main_rs.with_suffix('.old'), 'w') as f:
            f.write(old_contents)
    
    def reset_func(self, func):
        prCyan("Resetting changes.")
        # Replace the ".rs" and ".c" files with the ".old" files if they exist
        original_c_file = Path(self.c_code_dir, func['filename'])
        original_rust_file = Path(self.code_dir, 'src/main.rs')
        for file in [original_c_file, original_rust_file, self.bindgen_blocklist]:
            if file.with_suffix('.old').exists():
                shutil.copy(file.with_suffix('.old'), file)
                file.with_suffix('.old').unlink()

    def cleanup(self):
        cwd = os.getcwd()
        os.chdir(self.code_dir)
        cmd = 'rm -rf target'
        try:
            run(cmd)
        except RunException as e:
            # Read e and look for strings that say:
            # rm: cannot remove '<filename>': Device or resource busy
            # Get a list of these filenames
            for line in str(e).split('\n'):
                if 'cannot remove' in line and "Device or resource busy" in line:
                    filename = line.split('\'')[1]
                    try:
                        run(f'fuser -k {filename}')
                    except:
                        pass
                    try:
                        run(f'rm -rf {filename}')
                    except:
                        pass
            try:
                run(cmd)
            except RunException as e:
                prRed(f"Failed to fully cleanup {self.code_dir}")
        os.chdir(cwd)