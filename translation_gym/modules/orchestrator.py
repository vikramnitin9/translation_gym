from translation_gym.helpers import *
from translation_gym.modules.validator import Validator
from translation_gym.core.test_manager import TestManager

class Orchestrator:

    """
    Base class for orchestrators. This class is responsible for generating the units in a specific order
    and annotating them with the necessary dependencies for translation.
    """

    def __init__(self, source_manager, target_manager, logger):
        pass

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
        :param target_manager: The target manager
        :param instrumentation_results: [Optional] instrumentation results to include only covered functions
        :return: An iterator over the translation units
        """
        raise NotImplementedError("Subclasses must implement this method")


class DefaultOrchestrator(Orchestrator):

    def __init__(self, source_manager, target_manager, logger):
        self.source_manager = source_manager
        self.target_manager = target_manager
        self.logger = logger
        self.translation_map = {}

    def update_state(self, unit, translation):
        """
        Update the state of the orchestrator agent after a successful translation.
        :param unit: The translation unit
        :param translation: The translation result
        """
        # Right now we use fixed naming conventions to create this map,
        # assuming that the translator follows the same conventions.
        # But to make it more robust, we might want to detect the name from `translation`.
        if unit['type'] in ['functions', 'structs']:
            self.translation_map[unit['name']] = unit['name'] + "_rust"
        elif unit['type'] == 'globals':
            self.translation_map[unit['name']] = f"{''.join([s.capitalize() for s in unit['name'].split('_')])}Wrapper"
        else:
            raise ValueError(f"Unknown unit type: {unit['type']}")
        
        self.source_static_analysis = self.source_manager.get_static_analysis_results()
        self.target_static_analysis = self.target_manager.get_static_analysis_results()
        self.__rebuild_dependency_graph()
    
    def __lookup_unit(self, unit_name, unit_type, language):
        """
        Look up a unit in the static analysis results.
        :param unit_name: The name of the unit
        :param unit_type: The type of the unit (functions, structs, globals)
        :return: The unit if found, None otherwise
        """
        if language == 'source':
            units = [res for res in self.source_static_analysis[unit_type] if res['name'] == unit_name]
        elif language == 'target':
            units = [res for res in self.target_static_analysis[unit_type] if res['name'] == unit_name]
        else:
            raise ValueError("Invalid language specified")
        
        if len(units) == 0:
            return None
        if len(units) > 1:
            self.logger.log_failure(f"Multiple units found for {unit_name} of type {unit_type} in {language}.")
        
        unit_to_return = units[0]
        unit_to_return['type'] = unit_type

        return unit_to_return

    def __get_attributes(self, unit_name):
        """
        Get the attributes of a source unit from the static analysis results.
        Note: This method assumes that the static analysis results are up to date.
        `get_static_analysis_results` should be called before this method.
        """

        unit_type = self.dep_graph.nodes[unit_name]['type']
        unit = self.__lookup_unit(unit_name, unit_type, 'source') # Look it up in 'source' specifically
        if not unit:
            return None

        source = self.source_manager.extract_source(unit)
        unit['source'] = source

        # Now we get the imports that are already in the Rust file
        insertion_file = self.target_manager.get_insertion_file(unit)
        code_dir = self.target_manager.get_code_dir()
        file_candidates = [file for file in self.target_static_analysis['files'] if compare_fnames(file['filename'], insertion_file, code_dir)]
        if len(file_candidates) == 1:
            unit['imports'] = [imp['source'] for imp in file_candidates[0]['imports']]
        else:
            unit['imports'] = []
        
        # For translating globals, we need the binding information while performing translation
        if unit['type'] == 'globals':
            binding = self.__lookup_unit(unit['name'], 'globals', 'target')
            if binding is None:
                self.logger.log_failure(f"Could not find binding for global variable '{unit['name']}' in target code.")
                return None
            unit['binding'] = self.target_manager.extract_source(binding)

        # The remaining steps are only relevant for functions
        if unit['type'] != 'functions':
            return unit
        
        # Reset all the attributes that we will fill in later
        unit['functions'], unit['structs'], unit['globals'] = [], [], []
        
        # Get all the neighboring nodes in the dependency graph
        for dep_name in self.dep_graph.neighbors(unit_name):
            dep = self.dep_graph.nodes[dep_name]
            dep_info = {'name': dep_name}
            # If the dependency is in the source language, look for a binding to it from the target language
            if dep['language'] == 'source':
                binding = self.__lookup_unit(dep_name, dep['type'], 'target')
                if binding is not None and binding['foreign']:
                    dep_info['binding'] = self.target_manager.extract_source(binding)
            # Now look for a translated version of the dependency (irrespective of which language it is in)
            if dep_name in self.translation_map:
                translated_name = self.translation_map[dep_name]
                if dep['type'] == 'globals':
                    # Globals get translated to a wrapper struct in Rust
                    translated_unit = self.__lookup_unit(translated_name, 'structs', 'target')
                else:
                    translated_unit = self.__lookup_unit(translated_name, dep['type'], 'target')
                if translated_unit is not None:
                    if dep['type'] == 'functions':
                        # For functions, we only extract the signature for conciseness
                        dep_info['translated'] = translated_unit['signature']
                    else:
                        # For structs and globals, we extract the full source code
                        dep_info['translated'] = self.target_manager.extract_source(translated_unit)
            unit[dep['type']].append(dep_info)
        
        return unit
    
    def __rebuild_dependency_graph(self):
        """
        Rebuild the dependency graph based on the static analysis results of the source and target code.
        Note: This method assumes that the static analysis results are up to date.
        `get_static_analysis_results` should be called before this method.
        """

        self.dep_graph = nx.DiGraph()
        
        for unit_type in ['functions', 'structs', 'globals']:
            for unit in self.source_static_analysis[unit_type]:
                self.dep_graph.add_node(unit['name'], type=unit_type, language='source')

        for unit_type in ['functions', 'structs', 'globals']:
            for unit in self.target_static_analysis[unit_type]:
                # Skip foreign units, as they represent bindings to the source language
                if unit['foreign']:
                    continue
                self.dep_graph.add_node(unit['name'], type=unit_type, language='target')

        for func in self.source_static_analysis['functions'] + self.target_static_analysis['functions']:
            if 'foreign' in func and func['foreign']:
                continue
            qname = func['name']
            func_node = self.dep_graph.nodes[qname]
            for dep_type in ['functions', 'globals', 'structs']:
                for dep in func[dep_type]:
                    qdep = dep['name']
                    if qdep not in self.dep_graph:
                        # If the dependency is not in the graph, add it
                        self.dep_graph.add_node(qdep, type=dep_type, language=func_node['language'])
                    self.dep_graph.add_edge(qname, qdep)
        
        
        # Propagate global dependencies through the call graph ⏬
        new_edge_added = True
        while new_edge_added:
            new_edge_added = False
            for f in list(self.dep_graph.nodes):
                if self.dep_graph.nodes[f]['type'] != 'functions':
                    continue
                callees = list(self.dep_graph.successors(f))  # Freeze here
                for c in callees:
                    if self.dep_graph.nodes[c]['type'] != 'functions':
                        continue
                    if c not in self.translation_map:
                        continue
                    globals_used_by_c = list(self.dep_graph.successors(c))  # Freeze here too
                    for g in globals_used_by_c:
                        if self.dep_graph.nodes[g]['type'] != 'globals':
                            continue
                        if not self.dep_graph.has_edge(f, g):
                            self.dep_graph.add_edge(f, g)
                            new_edge_added = True
        self.logger.dump_graph(self.dep_graph)

    def unit_iter(self, instrumentation_results=None):
        self.source_static_analysis = self.source_manager.get_static_analysis_results()
        self.target_static_analysis = self.target_manager.get_static_analysis_results()

        self.__rebuild_dependency_graph()

        # We only want to translate functions that are reachable from main
        reachable_q = nx.descendants(self.dep_graph, 'main_0') | {'main_0'}
        subgraph   = self.dep_graph.subgraph(reachable_q)

        components = nx.weakly_connected_components(subgraph)
        assert len(list(components)) == 1

        try:
            unit_ordering = list(reversed(list(nx.topological_sort(subgraph))))
        except nx.NetworkXUnfeasible:
            unit_ordering = list(nx.dfs_postorder_nodes(subgraph, source='main_0'))
        
        for unit_name in unit_ordering:
            # The translator will call `update_state` if the translation is successful,
            # which gets the latest static analysis results and rebuilds the dependency graph.
            # So at this point, we can safely assume that the static analysis and graph are up to date.
            unit = self.__get_attributes(unit_name)
            if not unit:
                continue
            if unit['type'] == "functions" and instrumentation_results is not None:
                instrumentation_logs = [log for log in instrumentation_results if log['name'] == unit['name']]
                if len(instrumentation_logs) != 0:
                    unit['instrumentation'] = {'args': instrumentation_logs[0]['args'],
                                                'return': instrumentation_logs[0]['return']}
                else:
                    # Include only covered functions
                    continue
            yield unit
    
    def prune(self, validator: Validator, test_manager: TestManager):
        
        self.logger.log_status("Pruning dependency graph...")

        # First, handle global variables. For each "wrapper" struct in the target code that
        # we created, ask an LLM to rewrite it to use its internal `val` field instead of
        # accessing the global variable binding to the source language.
        # TODO: Implement this logic

        self.source_static_analysis = self.source_manager.get_static_analysis_results()
        self.target_static_analysis = self.target_manager.get_static_analysis_results()
        
        graph_changed = True

        while graph_changed:
            graph_changed = False
            # Find nodes with no incoming edges
            # # skip any bindgen‐only functions
            nodes_to_remove = []
            for node, in_degree in self.dep_graph.in_degree():
                if in_degree != 0:
                    continue
                meta = self.__lookup_unit(node, self.dep_graph.nodes[node]['type'], 'target')
                # skip if it came from bindings.rs or is a bindgen helper
                if meta is None:
                    continue
                if meta.get('foreign', False) or meta.get('filename', '').endswith('bindings.rs'):
                    continue
                nodes_to_remove.append(node)
            for node in nodes_to_remove:
                if node == "main":
                    # Don't prune the main function!
                    continue
                unit_type = self.dep_graph.nodes[node]['type']
                unit_lang = self.dep_graph.nodes[node]['language']
                if unit_lang == 'source':
                    continue # We don't prune source units
                # If the unit has been changed in the previous iteration, we make sure to
                # call get_static_analysis_results. So at this point, we can assume that the
                # static analysis results are up to date.
                unit = self.__lookup_unit(node, unit_type, unit_lang)
                if unit is not None:
                    self.logger.log_status(f"Removing unused unit: '{node}' of type '{unit_type}' in the {unit_lang} code.")
                    self.target_manager.save_state(unit)
                    self.target_manager.remove_unit(unit)
                    result = validator.compile_and_test(self.source_manager, self.target_manager, test_manager)
                    if not result['success']:
                        self.logger.log_failure(f"Failed to compile after removing {unit['name']}: {result['message']}")
                        self.target_manager.reset_unit(unit)
                    else:
                        self.logger.log_success(f"Successfully pruned {unit['name']}.")
                        self.source_static_analysis = self.source_manager.get_static_analysis_results()
                        self.target_static_analysis = self.target_manager.get_static_analysis_results()
                        graph_changed = True
            
            self.source_static_analysis = self.source_manager.get_static_analysis_results()
            self.target_static_analysis = self.target_manager.get_static_analysis_results()
            self.__rebuild_dependency_graph()
