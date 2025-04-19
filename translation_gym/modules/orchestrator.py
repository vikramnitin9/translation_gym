from translation_gym.helpers import *

class Orchestrator:

    """
    Base class for orchestrators. This class is responsible for orchestrating the translation process.
    """

    def function_iter(self, source_manager, instrumentation_results):
        """
        Iterate over the functions in the source code.
        :param source_manager: The source manager
        :return: An iterator over the functions
        """
        raise NotImplementedError("Subclasses must implement this method")


class DefaultOrchestrator(Orchestrator):

    def function_iter(self, source_manager, instrumentation_results):
        static_analysis_results = source_manager.get_static_analysis_results()
        # Build call graph of functions
        self.call_graph = nx.DiGraph()
        for func in static_analysis_results:
            if 'calledFunctions' not in func:
                # These are functions which were in the AST but not in the LLVM IR
                continue
            self.call_graph.add_node('"{}"'.format(func['name']))
            for called_func in func['calledFunctions']:
                self.call_graph.add_edge('"{}"'.format(func['name']), '"{}"'.format(called_func))

        # We only want to translate functions that are reachable from main
        reachable_nodes = nx.descendants(self.call_graph, '"main_0"') | {'"main_0"'}
        subgraph = self.call_graph.subgraph(reachable_nodes)
        components = nx.weakly_connected_components(subgraph)
        assert len(list(components)) == 1

        try:
            func_ordering = list(reversed(list(nx.topological_sort(subgraph))))
        except nx.NetworkXUnfeasible:
            func_ordering = list(nx.dfs_postorder_nodes(subgraph, source='"main_0"'))
        
        func_ordering = [f.strip('"') for f in func_ordering]

        for func_name in func_ordering:
            funcs = [f for f in static_analysis_results if f['name'] == func_name]
            if len(funcs) == 0:
                continue
            func = funcs[0]
            instrumentation_logs = [log for log in instrumentation_results if log['name'] == func_name]
            if len(instrumentation_logs) == 0:
                # Include only covered functions
                continue
            func['instrumentation'] = {'args': instrumentation_logs[0]['args'],
                                       'return': instrumentation_logs[0]['return']}
            yield func