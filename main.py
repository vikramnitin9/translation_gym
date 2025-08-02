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
    parser.add_argument('--prune_only', action='store_true', help='Run pruning on an existing output_dir that already contains translated code; do not translate anything.')

    args = parser.parse_args()

    datasets = json.loads(open('data/datasets.json').read())
    assert args.dataset in datasets, f"Dataset {args.dataset} not found in datasets.yaml"
    dataset = datasets[args.dataset]
    assert 'code_dir' in dataset, f"Code directory not specified for dataset {args.dataset}"


    if args.prune_only:
        if not Path(args.output_dir).exists():
            raise FileNotFoundError(f"{args.output_dir} does not exist – nothing to prune.")
    else:
        if Path(args.output_dir).exists():
            raise FileExistsError(
                f"Directory {args.output_dir} already exists. "
                "Remove it, choose a new one, or use --prune_only."
            )
    if args.prune_only:
        out_path = Path(args.output_dir)
        logger = Logger(out_path, args, verbose=args.verbose)

        # --- managers that point to EXISTING trees -------------------------------
        from translation_gym.core.source_manager import CSourceManager
        from translation_gym.core.target_manager import RustTargetManager
        from translation_gym.core.test_manager   import TestManager


        src_mgr = src_mgr = CSourceManager(out_path / "source", logger)

        tgt_mgr = RustTargetManager(
            out_path,  
            out_path / "source", 
            logger
        )

        src_mgr.setup()
        src_mgr.compile()

        tgt_mgr.cargo_bin_target = args.dataset 
        tgt_mgr.bindgen_blocklist = (out_path / "bindgen_blocklist.txt")
        tgt_mgr.bindgen_blocklist.touch(exist_ok=True)

        orchestrator = DefaultOrchestrator(src_mgr, tgt_mgr, logger)
        validator    = DefaultValidator(compile_attempts=2, logger=logger)

        test_manager = TestManager(test_docker=f"{args.dataset}:latest", logger=logger)


        orchestrator.source_static_analysis = src_mgr.get_static_analysis_results()
        orchestrator.target_static_analysis = tgt_mgr.get_static_analysis_results()
        orchestrator._DefaultOrchestrator__rebuild_dependency_graph()


        logger.log_status("Pruning translated code ...")
        orchestrator.prune(validator, test_manager)
        logger.log_status("Pruning complete")
        exit(0)


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