import argparse
import json

from translation_gym.core.translation_engine import TranslationEngine
from translation_gym.helpers import *

from translation_gym.modules.orchestrator import DefaultOrchestrator
from translation_gym.modules.translator import DefaultTranslator
from translation_gym.modules.validator import DefaultValidator
        
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

    orchestrator = DefaultOrchestrator()
    translator = DefaultTranslator(args.model)
    validator = DefaultValidator(compile_attempts=5) # In case compilation times out, how many times to retry

    engine = TranslationEngine(dataset=dataset,
                               output_dir=args.output_dir,
                               model=args.model,
                               num_attempts=args.num_attempts,
                               verbose=args.verbose)
    
    engine.run(translator=translator,
               orchestrator=orchestrator,
               validator=validator)
 
    engine.print_results()