from translation_gym.helpers import *

class TargetManager:

    def __init__(self, code_dir, src_build_path, logger):
        raise NotImplementedError("TargetManager is an abstract class and cannot be instantiated directly.")
    
    def get_code_dir(self):
        raise NotImplementedError("get_code_dir() not implemented")

    def get_static_analysis_results(self):
        raise NotImplementedError("get_static_analysis_results() not implemented")

    def get_func_by_name(self, func_name):
        raise NotImplementedError("get_func_by_name() not implemented")
    
    def replace_func(self, func, new_body):
        raise NotImplementedError("replace_func() not implemented")
    
    def reset_unit(self, func):
        raise NotImplementedError("reset_unit() not implemented")
    
    def cleanup(self):
        raise NotImplementedError("cleanup() not implemented")

    def setup(self):
        raise NotImplementedError("setup() not implemented")

    def set_target_name(self, target):
        raise NotImplementedError("set_target_name() not implemented")

    def get_instrumentation_dir(self):
        raise NotImplementedError("get_instrumentation_dir() not implemented")
    
    def get_executable(self):
        raise NotImplementedError("get_executable() not implemented")
    
    def compile(self, timeout):
        raise NotImplementedError("compile() not implemented")
    
    def insert_translation(self, func, translation):
        raise NotImplementedError("insert_translation() not implemented")
    
    def get_insertion_file(self, func):
        raise NotImplementedError("get_insertion_file() not implemented")
    

class RustTargetManager(TargetManager):

    def __init__(self, code_dir, src_build_path, logger):
        self.code_dir = code_dir
        self.src_build_path = src_build_path
        self.instrumentation_dir = Path(self.code_dir, 'instrumentation')
        self.instrumentation_dir.mkdir(parents=True, exist_ok=True)
        self.last_compile_time = 0
        self.static_analysis_results = None
        self.logger = logger
        self.modified = True

    def get_code_dir(self):
        return self.code_dir

    def setup(self):
        shutil.copytree('resources/rust_wrapper', self.code_dir, dirs_exist_ok=True)
        self.bindgen_blocklist = Path(self.code_dir, 'bindgen_blocklist.txt')
        if self.bindgen_blocklist.exists():
            self.bindgen_blocklist.unlink()
        self.bindgen_blocklist.touch()
        self.modified = True

    def set_target_name(self, target):
        cargo_toml_path = Path(self.code_dir, 'Cargo.toml')
        contents = f"""
            [package]
            name = "foo"
            version = "0.1.0"
            edition = "2021"
            build = "build.rs"

            [[bin]]
            path = "src/main.rs"
            name = "{target}"

            [dependencies]
            libc= "0.2"

            [build-dependencies]
            bindgen = "0.71.1"
            serde_json = "1.0.140"
            glob = "0.3.2"
        """
        contents = textwrap.dedent(contents)
        cargo_toml_path.write_text(contents)
        self.cargo_bin_target = target

    def get_static_analysis_results(self):
        self.compile()
        analysis_json_path = Path(self.code_dir)/'analysis.json'
        self.static_analysis_results = json.load(open(analysis_json_path, 'r'))
        return self.static_analysis_results
    
    def get_func_by_name(self, func_name):
        functions = self.get_static_analysis_results()
        for func in functions:
            if func['name'] == func_name:
                return func
        return None
    
    def replace_func(self, func, new_body):

        fpath = Path(os.path.join(self.code_dir, func['filename']))
        start_line = func['startLine']
        start_col = func['startCol']
        end_line = func['endLine']
        end_col = func['endCol']

        with open(fpath, 'r') as f:
            lines = f.readlines()

        before = lines[:start_line-1] + [lines[start_line-1][:start_col-1]]
        after = [lines[end_line-1][end_col:]] + lines[end_line:]
        new_contents = ''.join(before + [new_body] + after)

        with open(fpath, 'w') as f:
            f.write(new_contents)
        self.modified = True

    def get_executable(self):
        # The executable is created by `cargo parse`, 
        # which creates it in the target/<host>/debug directory
        command = "rustc -vV | grep '^host:' | awk '{ print $2 }'"
        result = run(command, timeout=20, logger=self.logger)
        host = result.strip()
        # Check if the executable exists
        executable = Path(self.code_dir, f'target/{host}/debug/{self.cargo_bin_target}')
        if not executable.exists():
            raise Exception(f"Executable not found at {executable}. Please compile the code first.")
        return executable

    def get_instrumentation_dir(self):
        return self.instrumentation_dir
    
    def compile(self, timeout=60):
        if not self.modified:
            return
        cwd = os.getcwd()
        cmd = 'cd {} && RUSTFLAGS="-Awarnings" C_BUILD_PATH="{}" cargo parse'.format(self.code_dir, self.src_build_path)
        try:
            run(cmd, timeout=timeout, logger=self.logger)
            self.last_compile_time = time.time()
        except RunException as e:
            raise CompileException(e)
        finally:
            os.chdir(cwd)
            self.modified = False
    
    def extract_source(self, unit):
        if not Path(unit['filename']).is_absolute():
            fpath = Path(os.path.join(self.code_dir, unit['filename']))
        else:
            fpath = Path(unit['filename'])
        start_line = unit['startLine']
        start_col = unit['startCol']
        end_line = unit['endLine']
        end_col = unit['endCol']

        with open(fpath, 'r') as f:
            lines = f.readlines()
        
        if start_line == end_line:
            # If the start and end lines are the same, just return that line
            return lines[start_line-1][start_col-1:end_col]
        # If the start and end lines are different, we need to concatenate the lines
        source = lines[start_line-1][start_col-1:]
        for i in range(start_line, end_line-1):
            source += lines[i]
        source += lines[end_line-1][:end_col]
        return source

    def insert_imports(self, unit, imports):

        insertion_file = self.get_insertion_file(unit)
        # Read the contents of the insertion file
        contents = insertion_file.read_text()
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

        insertion_file.write_text(new_contents)
        # De-duplicate imports
        try:
            run(f'cd {self.code_dir} && rustfmt --config imports_granularity=Crate src/main.rs')
        except:
            self.logger.log_failure("Rustfmt failed. There may be a syntax error in the generated code.")

        self.modified = True

    def insert_translation(self, unit, translation):

        if unit['type'] == 'functions':
            # Add the function to the bindgen blocklist
            with open(self.bindgen_blocklist, 'a') as f:
                f.write(f"{unit['name']}\n")

        insertion_file = self.get_insertion_file(unit)
        # Read the contents of the insertion file
        contents = insertion_file.read_text()

        # Insert the translation at the bottom
        insertion_file.write_text(contents + '\n' + translation)
        self.modified = True

    def get_insertion_file(self, func):
        # Get the file where the function will be inserted
        # Right now, this is always src/main.rs
        # But in the future, we may want a more modular approach
        return Path(self.code_dir, 'src/main.rs')

    def save_state(self, unit):
        if unit['type'] == 'functions':
            shutil.copy(self.bindgen_blocklist, self.bindgen_blocklist.with_suffix('.old'))
        fpath = self.get_insertion_file(unit)
        if not fpath.exists():
            raise FileNotFoundError(f"File {fpath} does not exist.")
        with open(fpath, 'r') as f:
            contents = f.read()
        with open(fpath.with_suffix('.old'), 'w') as f:
            f.write(contents)

    def reset_unit(self, unit):
        self.logger.log_status("Resetting changes.")
        # Replace the ".rs" files with the ".old" files if they exist
        files_to_restore = [self.get_insertion_file(unit)]
        if unit['type'] == 'functions':
            files_to_restore.append(self.bindgen_blocklist)
        for file in files_to_restore:
            if file.with_suffix('.old').exists():
                shutil.copy(file.with_suffix('.old'), file)
            else:
                raise FileNotFoundError(f"File {file.with_suffix('.old')} does not exist. Cannot reset unit {unit['name']}.")
        self.modified = True

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
                self.logger.log_failure(f"Failed to fully cleanup {self.code_dir}")
        finally:
            os.chdir(cwd)
            self.modified = True