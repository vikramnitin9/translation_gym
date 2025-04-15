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
    parser.add_argument('--dataset',        type=str,   default='toy',          help='Dataset identifier from datasets.yaml')
    parser.add_argument('--model',          type=str,   default='gpt4o-mini',   help='Model to use for translation')
    parser.add_argument('--num_attempts',   type=int,   default=5,              help='Number of attempts to translate each function')
    parser.add_argument('--output_dir',     type=str,   default='output/translation', help='Directory to write the output')
    parser.add_argument('--verbose',        action='store_true',                help='Enable verbose output')
    args = parser.parse_args()

    datasets = json.loads(open('data/datasets.json').read())
    assert args.dataset in datasets, f"Dataset {args.dataset} not found in datasets.yaml"
    dataset = datasets[args.dataset]
    assert 'code_dir' in dataset, f"Code directory not specified for dataset {args.dataset}"

    orchestrator = Orchestrator()
    translator = Translator(args.model)
    validator = Validator(compile_attempts=5) # In case compilation times out, how many times to retry

    engine = TranslationEngine(dataset=dataset,
                               output_dir=args.output_dir,
                               model=args.model,
                               num_attempts=args.num_attempts,
                               verbose=args.verbose)

    engine.run(translator=translator,
               orchestrator=orchestrator,
               validator=validator)