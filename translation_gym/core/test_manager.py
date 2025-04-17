from translation_gym.helpers import *

class TestManager:

    def __init__(self, test_docker: str, verbose: bool = False):
        self.test_docker    = test_docker
        self.verbose        = verbose

    def run_tests(self, executable: Path):
        prCyan("Running tests against the following executable: {}".format(executable))

        cmd = f"docker run -it -v {executable}:/executable/toy {self.test_docker}"

        try:
            run(cmd, self.verbose)
            if self.verbose:
                prGreen(f"Test passed")
            return {'status': 'passed'}
        except RunException as e:
            if self.verbose:
                prRed(f"Test failed")
                prLightGray(e)
            return {'status': 'failed', 'error': str(e)}

        return self.status