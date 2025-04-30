from translation_gym.helpers import *

class SourceManager:

    def __init__(self, code_dir):
        raise NotImplementedError("SourceManager is an abstract class and cannot be instantiated directly.")

    def get_static_analysis_results(self):
        raise NotImplementedError("get_static_analysis_results() not implemented")
    
    def reset_func(self, func):
        raise NotImplementedError("reset_func() not implemented")

    def cleanup(self):
        raise NotImplementedError("cleanup() not implemented")

    def get_build_path(self):
        raise NotImplementedError("get_build_path() not implemented")

    def get_bin_target(self):
        raise NotImplementedError("get_bin_target() not implemented")

    def extract_body(self, func):
        raise NotImplementedError("extract_body() not implemented")

    def insert_translation(self, func, translation):
        raise NotImplementedError("insert_translation() not implemented")

    def comment_out(self, func):
        raise NotImplementedError("comment_out() not implemented")


class CManager(SourceManager):

    def __init__(self, code_dir):
        self.code_dir = code_dir
        self.last_compile_time = 0
        self.static_analysis_results = None
    
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
            prGreen(f"Found executable target: {target}")
        os.chdir(cwd)
        return target

    def compile(self, timeout=60, verbose=False):
        cwd = os.getcwd()
        os.chdir(self.code_dir)
        compile_commands_json = Path(self.code_dir)/'compile_commands.json'
        # Note - do not re-generate compile_commands.json if it already exists
        if not compile_commands_json.exists():
            # Get the output of bear --version
            try:
                bear_version = run("bear --version", verbose=verbose)
            except RunException as e:
                raise CompileException(e)
            version = bear_version.split()[1]
            major, _, _ = version.split('.')
            if int(major) >= 3:
                cmd = 'make clean && bear -- make'
            else:
                cmd = "make clean && bear make"
            try:
                run(cmd, verbose=verbose)
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
            run('cd {} && {}/parsec *.c'.format(self.code_dir, parsec_build_dir), timeout=timeout, verbose=verbose)
            self.last_compile_time = time.time()
        except RunException as e:
            raise CompileException(e)
        finally:
            os.chdir(cwd)
    
    def get_build_path(self):
        return Path(self.code_dir)
    
    def get_static_analysis_results(self):
        last_modified_time = get_last_modified_time(self.code_dir, ".c")
        if last_modified_time > self.last_compile_time:
            prCyan("Code has changed, re-compiling.")
            self.compile()
            functions_json_path = Path(self.code_dir)/'functions.json'
            self.static_analysis_results = json.load(open(functions_json_path, 'r'))
        elif self.static_analysis_results is None:
            functions_json_path = Path(self.code_dir)/'functions.json'
            self.static_analysis_results = json.load(open(functions_json_path, 'r'))
        return self.static_analysis_results

    def get_instrumentation_dir(self):
        return self.instrumentation_dir
    
    def extract_body(self, func):
        fpath = Path(os.path.join(self.code_dir, func['filename']))
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

    def comment_out(self, func):

        fpath = Path(os.path.join(self.code_dir, func['filename']))
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
    
    def reset_func(self, func):
        prCyan("Resetting changes.")
        # Replace the ".c" file with the ".old" file if it exists
        file = Path(self.code_dir, func['filename'])
        if file.with_suffix('.old').exists():
            shutil.copy(file.with_suffix('.old'), file)
            file.with_suffix('.old').unlink()

    def cleanup(self):
        cwd = os.getcwd()
        os.chdir(self.code_dir)
        cmd = 'make clean && rm -f compile_commands.json && rm -f libfoo.a'
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