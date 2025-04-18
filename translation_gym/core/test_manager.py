from translation_gym.helpers import *
from translation_gym.core.source_manager import SourceManager

class TestManager:

    def __init__(self, test_docker: str, verbose: bool = False):
        self.test_docker    = test_docker
        self.verbose        = verbose
    
    def collect_instrumentation_results(self, source_manager: SourceManager):
        instrumentation_dir = source_manager.get_instrumentation_dir()
        # Check if the instrumentation.json file exists
        instrumentation_json = instrumentation_dir/"instrumented.json"
        if not instrumentation_json.exists():
            return None
        return safe_load_json(instrumentation_json)
    
    def run_tests(self, source_manager: SourceManager):
        executable = source_manager.get_executable()
        if not executable:
            prRed("No executable found. Please compile the code first.")
            return {'status': 'failed', 'error': 'No executable found'}
        executable_host_path = source_manager.to_host_path(executable)
        if not executable_host_path:
            prRed("Executable is not in a mounted directory.")
            return {'status': 'failed', 'error': 'Excutable is not in a mounted directory.'}

        instrumentation_dir = source_manager.get_instrumentation_dir()
        instrumentation_host_path = source_manager.to_host_path(instrumentation_dir)
        if not instrumentation_host_path:
            prRed("Instrumentation directory is not in a mounted directory.")
        
        # Delete instrumentation_dir/"instrumentation.json" if it exists
        instrumentation_json = instrumentation_dir/"instrumented.json"
        if instrumentation_json.exists():
            instrumentation_json.unlink()

        prCyan("Running tests against the following executable: {}".format(executable_host_path))
        # Get the name of the executable
        exec_name = executable_host_path.name
        cmd = f"docker run -it \
                -v {executable_host_path}:/executable/{exec_name}\
                -v {instrumentation_host_path}:/instrumentation \
                -e INSTRUMENTATION_PATH=/instrumentation \
                    {self.test_docker}"
        if self.verbose:
            prCyan(f"Running command: {cmd}")

        try:
            run(cmd, self.verbose)
            instrumentation = self.collect_instrumentation_results(source_manager)
            return {'status': 'passed', 'instrumentation': instrumentation}
        except RunException as e:
            return {'status': 'failed', 'error': str(e), 'instrumentation': instrumentation}