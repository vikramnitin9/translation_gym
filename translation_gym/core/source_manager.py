from translation_gym.helpers import *

class SourceManager:

    def __init__(self, code_dir, logger):
        raise NotImplementedError("SourceManager is an abstract class and cannot be instantiated directly.")
    
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

    def get_build_path(self):
        raise NotImplementedError("get_build_path() not implemented")

    def get_bin_target(self):
        raise NotImplementedError("get_bin_target() not implemented")

    def extract_body(self, func):
        raise NotImplementedError("extract_body() not implemented")

    def remove_func(self, func):
        raise NotImplementedError("remove_func() not implemented")


class CSourceManager(SourceManager):

    def __init__(self, code_dir, logger):
        self.code_dir = Path(code_dir)
        self.last_compile_time = 0
        self.static_analysis_results = None
        self.logger = logger
        self.modified = True

    def get_code_dir(self):
        return self.code_dir
    
    def get_bin_target(self):

        cwd = os.getcwd()
        os.chdir(self.code_dir)

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
            self.logger.log_success(f"Found executable target: {target}")
        os.chdir(cwd)
        return target

    def compile(self, instrument=False, timeout=60):
        if not self.modified:
            return
        cwd = os.getcwd()
        os.chdir(self.code_dir)
        compile_commands_json = Path(self.code_dir)/'compile_commands.json'
        # Note - do not re-generate compile_commands.json if it already exists
        if not compile_commands_json.exists():
            # Get the output of bear --version
            try:
                bear_version = run("bear --version", logger=self.logger)
            except RunException as e:
                raise CompileException(e)
            version = bear_version.split()[1]
            major, _, _ = version.split('.')
            if int(major) >= 3:
                cmd = 'make clean && bear -- make'
            else:
                cmd = "make clean && bear make"
            try:
                run(cmd, logger=self.logger)
            except RunException as e:
                raise CompileException(e)
        # Check if the compile_commands.json file exists
        if not compile_commands_json.exists():
            raise CompileException("Error: compile_commands.json not found. Something wrong.")
        # Check if PARSEC_BUILD_DIR is set
        parsec_build_dir = os.environ.get('PARSEC_BUILD_DIR')
        if parsec_build_dir is None:
            raise CompileException("Error: $PARSEC_BUILD_DIR not set.")
        try:
            instr = "true" if instrument else "false"
            run(f'cd {self.code_dir} && {parsec_build_dir}/parsec --rename-main=true --add-instr={instr} *.c', timeout=timeout, logger=self.logger)
            self.last_compile_time = time.time()
        except RunException as e:
            raise CompileException(e)
        finally:
            os.chdir(cwd)
            self.modified = False

    def fix_globals(self):
      
        analysis = self.get_static_analysis_results()
        globals = [glob['name'] for glob in analysis['globals']]

        for glob_name in globals:
            analysis = self.get_static_analysis_results()
            candidates = [glob for glob in analysis['globals'] if glob['name'] == glob_name]
            if len(candidates) != 1:
                self.logger.log_failure(f"[fix_globals] Skipping {glob_name}")
                continue
            glob = candidates[0]
            try:
                source = self.extract_source(glob)
            except:
                # Sometimes, extract_source fails due to macro expansions
                self.logger.log_failure(f"[fix_globals] Failed to extract source for {glob['name']}")
                continue
            if glob['isStatic']:
                assert 'static ' in source, f"Expected 'static ' in {glob['name']} but got {source}"
                source = source.replace('static ', '', 1)
            # Extract everything until '='
            extern_decl = f"extern {source.split('=')[0].strip()};\n"
            source = extern_decl + source
            self.save_state(glob)
            self.replace_unit(glob, source)
            try:
                self.compile()
            except CompileException as e:
                self.logger.log_failure(f"[fix_globals] Failed to compile after patching {glob['name']}")
                self.reset_unit(glob)
                continue
            self.logger.log_success(f"[fix_globals] Fixed global {glob['name']} in {glob['filename']}")

    def get_build_path(self):
        return Path(self.code_dir)
    
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

    def get_instrumentation_dir(self):
        return self.instrumentation_dir
    
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

    def save_state(self, unit):
        # Save the current state of the file
        fpath = Path(os.path.join(self.code_dir, unit['filename']))
        if not fpath.exists():
            raise FileNotFoundError(f"File {fpath} does not exist.")
        with open(fpath, 'r') as f:
            contents = f.read()
        with open(fpath.with_suffix('.old'), 'w') as f:
            f.write(contents)
        self.logger.log_status(f"Saved state of {fpath} to {fpath.with_suffix('.old')}")

    def replace_unit(self, unit, new_body):

        fpath = Path(os.path.join(self.code_dir, unit['filename']))
        start_line = unit['startLine']
        start_col = unit['startCol']
        end_line = unit['endLine']
        end_col = unit['endCol']

        with open(fpath, 'r') as f:
            lines = f.readlines()

        before = lines[:start_line-1] + [lines[start_line-1][:start_col-1]]
        after = [lines[end_line-1][end_col:]] + lines[end_line:]
        new_contents = ''.join(before + [new_body] + after)

        with open(fpath, 'w') as f:
            f.write(new_contents)

        self.modified = True

    def remove_func(self, func):
        assert func['type'] == 'functions'

        fpath = Path(os.path.join(self.code_dir, func['filename']))
        start_line = func['startLine']
        start_col = func['startCol']
        end_line = func['endLine']
        end_col = func['endCol']

        with open(fpath, 'r') as f:
            lines = f.readlines()

        new_lines = lines[:start_line-1]

        # We need to keep the function declaration visible to other functions that may call it
        new_lines.append(lines[start_line-1][:start_col-1] + '\n')
        new_lines.append(func['signature'] + ';\n')
        new_lines.append(lines[end_line-1][end_col:] + '\n')
        new_lines += lines[end_line:]

        with open(fpath, 'w') as f:
            f.write(''.join(new_lines))

        self.modified = True

    def comment_out(self, unit):

        fpath = Path(os.path.join(self.code_dir, unit['filename']))
        start_line = unit['startLine']
        start_col = unit['startCol']
        end_line = unit['endLine']
        end_col = unit['endCol']

        with open(fpath, 'r') as f:
            lines = f.readlines()

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

        self.modified = True
    
    def reset_unit(self, unit):
        self.logger.log_status("Resetting changes.")
        # Replace the ".c" file with the ".old" file if it exists
        file = Path(self.code_dir, unit['filename'])
        if file.with_suffix('.old').exists():
            shutil.copy(file.with_suffix('.old'), file)
        else:
            raise FileNotFoundError(f"File {file.with_suffix('.old')} does not exist. Cannot reset unit.")
        self.modified = True

    def cleanup(self):
        cwd = os.getcwd()
        os.chdir(self.code_dir)
        cmd = 'make clean && rm -f libfoo.a'
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