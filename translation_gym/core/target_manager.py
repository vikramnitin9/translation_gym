from translation_gym.helpers import *

class TargetManager:

    def __init__(self, code_dir, src_build_path):
        raise NotImplementedError("TargetManager is an abstract class and cannot be instantiated directly.")

    def get_static_analysis_results(self):
        raise NotImplementedError("get_static_analysis_results() not implemented")
    
    def reset_func(self, func):
        raise NotImplementedError("reset_func() not implemented")
    
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
    
    def compile(self, timeout, verbose):
        raise NotImplementedError("compile() not implemented")
    
    def insert_translation(self, translation):
        raise NotImplementedError("insert_translation() not implemented")
    

class RustManager(TargetManager):

    def __init__(self, code_dir, src_build_path):
        self.code_dir = code_dir
        self.src_build_path = src_build_path
        self.instrumentation_dir = Path(self.code_dir, 'instrumentation')
        self.instrumentation_dir.mkdir(parents=True, exist_ok=True)
        self.last_compile_time = 0
        self.static_analysis_results = None

    def setup(self):
        shutil.copytree('resources/rust_wrapper', self.code_dir, dirs_exist_ok=True)
        self.bindgen_blocklist = Path(self.code_dir, 'bindgen_blocklist.txt')
        if self.bindgen_blocklist.exists():
            self.bindgen_blocklist.unlink()
        self.bindgen_blocklist.touch()

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
        last_modified_time = get_last_modified_time(self.code_dir, ".rs")
        if last_modified_time > self.last_compile_time:
            prCyan("Code has changed, re-compiling.")
            self.compile()
            functions_json_path = Path(self.code_dir)/'functions.json'
            self.static_analysis_results = json.load(open(functions_json_path, 'r'))
        elif self.static_analysis_results is None:
            functions_json_path = Path(self.code_dir)/'functions.json'
            self.static_analysis_results = json.load(open(functions_json_path, 'r'))
        return self.static_analysis_results

    def get_executable(self):
        # The executable is created by `cargo parse`, 
        # which creates it in the target/<host>/debug directory
        command = "rustc -vV | grep '^host:' | awk '{ print $2 }'"
        result = run(command, timeout=20, verbose=False)
        host = result.strip()
        # Check if the executable exists
        executable = Path(self.code_dir, f'target/{host}/debug/{self.cargo_bin_target}')
        if not executable.exists():
            raise Exception(f"Executable not found at {executable}. Please compile the code first.")
        return executable

    def get_instrumentation_dir(self):
        return self.instrumentation_dir
    
    def compile(self, timeout=60, verbose=False):
        cwd = os.getcwd()
        cmd = 'cd {} && RUSTFLAGS="-Awarnings" C_BUILD_PATH="{}" cargo parse'.format(self.code_dir, self.src_build_path)
        try:
            run(cmd, timeout=timeout, verbose=verbose)
            self.last_compile_time = time.time()
        except RunException as e:
            raise CompileException(e)
        finally:
            os.chdir(cwd)

    def insert_translation(self, func_name, translation):

        # Add the function to the bindgen blocklist
        shutil.copy(self.bindgen_blocklist, self.bindgen_blocklist.with_suffix('.old'))
        with open(self.bindgen_blocklist, 'a') as f:
            f.write(f"{func_name}\n")

        function_trans = translation['func']
        wrapper = translation['wrapper']
        imports = translation['imports'] if 'imports' in translation else ''

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
        # Replace the ".rs" files with the ".old" files if they exist
        original_rust_file = Path(self.code_dir, 'src/main.rs')
        for file in [original_rust_file, self.bindgen_blocklist]:
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