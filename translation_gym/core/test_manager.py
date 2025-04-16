from translation_gym.helpers import *

class TestManager:

    def __init__(self, test_scripts: List[Path], setup_script: Path, verbose: bool = False):
        self.test_scripts   = test_scripts
        self.setup_script   = setup_script
        self.status         = []
        self.verbose        = verbose

    def run_tests(self, executable: Path, stop_on_failure=False):
        prCyan("Running tests against the following executable: {}".format(executable))

        if self.setup_script is not None:
            # Run the setup script
            cmd = f'PATH="{executable.parent.absolute()}:$PATH" bash {self.setup_script}'
            try:
                run(cmd)
                if self.verbose:
                    prGreen(f"Setup script passed: {self.setup_script}")
            except RunException as e:
                if self.verbose:
                    prRed(f"Setup script failed: {self.setup_script}")
                raise e

        self.status = []
        # Run each test file
        for test_path in self.test_scripts:
            # Run the test file
            cmd = f'PATH="{executable.parent.absolute()}:$PATH" bash {test_path}'
            try:
                run(cmd)
                if self.verbose:
                    prGreen(f"Test passed: {test_path}")
                self.status.append({'test': test_path, 'status': 'passed'})
            except RunException as e:
                if self.verbose:
                    prRed(f"Test failed: {test_path}")
                self.status.append({'test': test_path, 'status': 'failed', 'error': str(e)})
                if stop_on_failure:
                    # Stop running tests if one fails
                    break
        return self.status
    
    def passed(self):
        # Check if all tests passed
        return all([s['status'] == 'passed' for s in self.status])
    
    def set_test_scripts(self, test_scripts):
        # Sometimes we want to use only a subset of the test scripts
        self.test_scripts = test_scripts
