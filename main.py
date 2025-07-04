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

    if Path(args.output_dir).exists():
        raise FileExistsError(f"Directory {args.output_dir} already exists. Please remove it before running the script.")

    logger = Logger(args.output_dir, args, verbose=args.verbose)

    engine = TranslationEngine(dataset=dataset,
                               output_dir=args.output_dir,
                               num_attempts=args.num_attempts,
                               logger=logger)
    
    source_manager = engine.get_source_manager()
    target_manager = engine.get_target_manager()

    orchestrator = DefaultOrchestrator(source_manager, target_manager, logger)
    translator = DefaultTranslator(args.model, logger)
    validator = DefaultValidator(compile_attempts=2, logger=logger) # In case compilation times out, how many times to retry

    engine.run(translator=translator,
               orchestrator=orchestrator,
               validator=validator)
 
    engine.print_results()