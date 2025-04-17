from translation_gym.helpers import *

class TestManager:

    def __init__(self, test_docker: str, verbose: bool = False):
        self.test_docker    = test_docker
        self.verbose        = verbose

    def run_tests(self, executable: Path):
        prCyan("Running tests against the following executable: {}".format(executable))
        # Get the name of the executable
        exec_name = executable.name
        cmd = f"docker run -it -v {executable}:/executable/{exec_name} {self.test_docker}"
        if self.verbose:
            prCyan(f"Running command: {cmd}")

        try:
            run(cmd, self.verbose)
            return {'status': 'passed'}
        except RunException as e:
            return {'status': 'failed', 'error': str(e)}

        return self.status