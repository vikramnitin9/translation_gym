from translation_gym.helpers import *

class Orchestrator:

    """
    Base class for orchestrators. This class is responsible for generating the units in a specific order
    and annotating them with the necessary dependencies for translation.
    """

    def update_state(self, unit, translation):
        """
        Update the state of the source and target managers after a successful translation.
        :param unit: The translation unit
        :param translation: The translation result
        """
        raise NotImplementedError("Subclasses must implement this method")

    def unit_iter(self, source_manager, target_manager, instrumentation_results):
        """
        Iterate over the translation units in the source code.
        :param source_manager: The source manager
        :return: An iterator over the translation units
        """
        raise NotImplementedError("Subclasses must implement this method")


class DefaultOrchestrator(Orchestrator):

    def __init__(self, logger):
        self.logger = logger

    def update_state(self, unit, translation):
        """
        Update the state of the orchestrator agent after a successful translation.
        :param unit: The translation unit
        :param translation: The translation result
        """
        # This translation logic doesn't require a state update,
        # but it is possible that a different orchestrator might need to keep track of the state.
        pass

    def attach_dependencies(self, unit, source_manager, target_manager, instrumentation_results=None):
        # We first need to compile the source to get the most up-to-date static library
        source_manager.compile()
        # Then we get the Rust static analysis results
        rust_static_analysis = target_manager.get_static_analysis_results()

        source = source_manager.extract_source(unit)
        unit['source'] = source

        # Now we need to get the imports that are already in the Rust file
        insertion_file = target_manager.get_insertion_file(unit)
        code_dir = target_manager.get_code_dir()
        file_candidates = [file for file in rust_static_analysis['files'] if compare_fnames(file['filename'], insertion_file, code_dir)]
        if len(file_candidates) == 1:
            unit['imports'] = [imp['source'] for imp in file_candidates[0]['imports']]
        else:
            unit['imports'] = []

        if unit['type'] != 'functions':
            return

        if (unit['type'] == "functions") and (instrumentation_results is not None):
            instrumentation_logs = [log for log in instrumentation_results if log['name'] == unit['name']]
            if len(instrumentation_logs) != 0:
                unit['instrumentation'] = {'args': instrumentation_logs[0]['args'],
                                           'return': instrumentation_logs[0]['return']}

        # This is the part where we collect info about the called functions.
        for i, called_func in enumerate(unit['calledFunctions']):
            translated_rust_fns = [f for f in rust_static_analysis['functions'] if f['name'] == (called_func['name'] + "_rust")]
            if len(translated_rust_fns) != 0:
                assert len(translated_rust_fns) == 1
                unit['calledFunctions'][i]['translated'] = translated_rust_fns[0]['signature']

            rust_ffi_bindings = [f for f in rust_static_analysis['functions'] if f['name'] == called_func['name']]
            if len(rust_ffi_bindings) != 0:
                assert len(rust_ffi_bindings) == 1
                unit['calledFunctions'][i]['binding'] = rust_ffi_bindings[0]['signature']

        # Now we get information about the globals and structs
        for i, struct in enumerate(unit['structs']):
            translated_rust_structs = [s for s in rust_static_analysis['structs'] if s['name'] == (struct['name'] + "_rust")]
            if len(translated_rust_structs) != 0:
                assert len(translated_rust_structs) == 1
                source = target_manager.extract_source(translated_rust_structs[0])
                unit['structs'][i]['translated'] = source
            
            rust_ffi_bindings = [s for s in rust_static_analysis['structs'] if s['name'] == struct['name']]
            if len(rust_ffi_bindings) != 0:
                assert len(rust_ffi_bindings) == 1
                source = target_manager.extract_source(rust_ffi_bindings[0])
                unit['structs'][i]['binding'] = source
        
        for i, glob in enumerate(unit['globals']):
            rust_ffi_bindings = [g for g in rust_static_analysis['globals'] if g['name'] == glob['name']]
            if len(rust_ffi_bindings) != 0: 
                assert len(rust_ffi_bindings) == 1
                source = target_manager.extract_source(rust_ffi_bindings[0])
                unit['globals'][i]['binding'] = source


    def unit_iter(self, source_manager, target_manager, instrumentation_results=None):
        self.source_manager = source_manager
        self.target_manager = target_manager

        self.static_analysis_results = self.source_manager.get_static_analysis_results()
        dep_graph = nx.DiGraph()

        for func in self.static_analysis_results['functions']:
            qname = func['name']
            dep_graph.add_node(qname, type='functions')

            # 1) function → callees
            for callee in func['calledFunctions']:
                qcallee = callee['name']
                dep_graph.add_node(qcallee, type='functions')
                dep_graph.add_edge(qname, qcallee)

            # # 2) function → globals 
            # for glob in func['globals']:
            #     qglob = glob['name']
            #     dep_graph.add_node(qglob, type='global')
            #     dep_graph.add_edge(qname, qglob)

             # 3) function → structs
            for st in func['structs']:
                qstruct = st['name']
                dep_graph.add_node(qstruct, type='structs')
                dep_graph.add_edge(qname, qstruct)

        # We only want to translate functions that are reachable from main
        reachable_q = nx.descendants(dep_graph, 'main_0') | {'main_0'}
        subgraph   = dep_graph.subgraph(reachable_q)

        components = nx.weakly_connected_components(subgraph)
        assert len(list(components)) == 1

        try:
            unit_ordering = list(reversed(list(nx.topological_sort(subgraph))))
        except nx.NetworkXUnfeasible:
            unit_ordering = list(nx.dfs_postorder_nodes(subgraph, source='main_0'))
        
        for unit_name in unit_ordering:
            unit_type = subgraph.nodes[unit_name]['type']
            units = [res for res in self.static_analysis_results[unit_type] if res['name'] == unit_name]
            if len(units) == 0:
                continue
            unit = units[0]
            unit['type'] = unit_type
            self.attach_dependencies(unit, source_manager, target_manager, instrumentation_results)
            if unit_type == "functions" and instrumentation_results is not None and 'instrumentation' not in unit:
                # Include only covered functions
                continue
            yield unit
