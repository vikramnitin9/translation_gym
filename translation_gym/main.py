import argparse
import shutil
import os
from pathlib import Path
import subprocess
import json
import networkx as nx
import datetime
from typing import List, Dict, Tuple

from translation_gym.orchestrator import Orchestrator
from translation_gym.translator import Translator
from translation_gym.validator import Validator
from translation_gym.translation_engine import TranslationEngine
from translation_gym.helpers import *
        
if __name__ == '__main__':

    parser = argparse.ArgumentParser(description='Translate code snippets to idiomatic Rust')
    parser.add_argument('--code_dir',       type=str,   required=True,          help='Root directory of the code to translate')
    parser.add_argument('--test_dir',       type=str,   default='',             help='Root directory of the test scripts')
    parser.add_argument('--model',          type=str,   default='gpt4o-mini',   help='Model to use for translation')
    parser.add_argument('--num_attempts',   type=int,   default=5,              help='Number of attempts to translate each function')
    parser.add_argument('--verbose',        action='store_true',                help='Enable verbose output')
    args = parser.parse_args()

    code_dir = Path(args.code_dir.rstrip('/'))
    assert Path(code_dir).exists(), f"Code directory {code_dir} does not exist"

    test_dir = Path(args.test_dir.rstrip('/'))
    if test_dir != '':
        assert Path(test_dir).exists(), f"Test directory {test_dir} does not exist"

    orchestrator = Orchestrator()
    translator = Translator(args.model)
    validator = Validator(compile_attempts=5) # In case compilation times out, how many times to retry

    engine = TranslationEngine(code_dir=code_dir,
                               test_dir=test_dir,
                               model=args.model,
                               num_attempts=args.num_attempts,
                               verbose=args.verbose)

    engine.run(translator=translator,
               orchestrator=orchestrator,
               validator=validator)