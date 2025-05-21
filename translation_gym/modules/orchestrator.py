from translation_gym.helpers import *

class Orchestrator:

    """
    Base class for orchestrators. This class is responsible for orchestrating the translation process.
    """

    def unit_iter(self, source_manager, instrumentation_results):
        """
        Iterate over the translation units in the source code.
        :param source_manager: The source manager
        :return: An iterator over the translation units
        """
        raise NotImplementedError("Subclasses must implement this method")


class DefaultOrchestrator(Orchestrator):

    def __init__(self, logger):
        self.logger = logger

    def unit_iter(self, source_manager, instrumentation_results=None):
        static_analysis_results = source_manager.get_static_analysis_results()
        dep_graph = nx.DiGraph()

        for func in static_analysis_results['functions']:
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
            result = [res for res in static_analysis_results[unit_type] if res['name'] == unit_name]
            if len(result) == 0:
                continue
            result = result[0]
            if (unit_type == "functions") and (instrumentation_results is not None):
                instrumentation_logs = [log for log in instrumentation_results if log['name'] == unit_name]
                if len(instrumentation_logs) == 0:
                    # Include only covered functions
                    continue
                result['instrumentation'] = {'args': instrumentation_logs[0]['args'],
                                            'return': instrumentation_logs[0]['return']}
            result['type'] = unit_type
            yield result
            # We have to re-run static analysis to get the most up-to-date results
            static_analysis_results = source_manager.get_static_analysis_results()
