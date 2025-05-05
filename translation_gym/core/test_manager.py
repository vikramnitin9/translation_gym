from translation_gym.helpers import *
from translation_gym.core.target_manager import TargetManager

class TestManager:

    def __init__(self, test_docker: str, logger: Logger):
        self.test_docker    = test_docker
        self.logger         = logger
    
    def collect_instrumentation_results(self, target_manager: TargetManager):
        instrumentation_dir = target_manager.get_instrumentation_dir()
        # Check if the instrumentation.json file exists
        instrumentation_json = instrumentation_dir/"instrumented.json"
        if not instrumentation_json.exists():
            return None
        return safe_load_json(instrumentation_json)
    
    def run_tests(self, target_manager: TargetManager):
        executable = target_manager.get_executable()
        if not executable:
            self.logger.log_failure("No executable found. Please compile the code first.")
            return {'status': 'failed', 'error': 'No executable found'}
        executable_host_path = to_host_path(executable)
        if not executable_host_path:
            self.logger.log_failure("Executable is not in a mounted directory.")
            return {'status': 'failed', 'error': 'Excutable is not in a mounted directory.'}

        instrumentation_dir = target_manager.get_instrumentation_dir()
        instrumentation_host_path = to_host_path(instrumentation_dir)
        if not instrumentation_host_path:
            self.logger.log_failure("Instrumentation directory is not in a mounted directory.")
        
        # Delete instrumentation_dir/"instrumentation.json" if it exists
        instrumentation_json = instrumentation_dir/"instrumented.json"
        if instrumentation_json.exists():
            instrumentation_json.unlink()

        self.logger.log_status("Running tests against the following executable: {}".format(executable_host_path))
        # Get the name of the executable
        exec_name = executable_host_path.name
        cmd = f"docker run -it \
                -v {executable_host_path}:/executable/{exec_name}\
                -v {instrumentation_host_path}:/instrumentation \
                    {self.test_docker}"
        try:
            run(cmd, timeout=120, logger=self.logger)
            instrumentation = self.collect_instrumentation_results(target_manager)
            return {'status': 'passed', 'instrumentation': instrumentation}
        except RunException as e:
            instrumentation = self.collect_instrumentation_results(target_manager)
            return {'status': 'failed', 'error': str(e), 'instrumentation': instrumentation}