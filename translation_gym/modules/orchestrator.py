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

    def __init__(self, logger):
        self.logger = logger

    def function_iter(self, source_manager, instrumentation_results=None):
        static_analysis_results = source_manager.get_static_analysis_results()
        # Build call graph of functions
        struct_names = {s['name'] for s in static_analysis_results['structures']}
        self.dep_graph = nx.DiGraph()

        for func in static_analysis_results['functions']:
            qname = '"{}"'.format(func['name'])
            self.dep_graph.add_node(qname)

            # 1) calls → function
            for callee in func.get('calledFunctions', []):
                cname = callee['name']
                qcallee = '"{}"'.format(callee['name'])
                self.dep_graph.add_node(qcallee)
                target = next((f for f in static_analysis_results['functions']
                                    if f['name'] == cname), None)
                passed = []
                if target:
                      for typ in target.get('argTypes', []):
                          for sn in struct_names:
                              if sn in typ and sn not in passed: 
                                   passed.append(sn)
                                   break
                self.dep_graph.add_edge(qname, qcallee, passed_structs=passed)

            # 2) globals → function
            for glob in func.get('globals', []):
                qglob = '"{}"'.format(glob['name'])
                self.dep_graph.add_node(qglob)
                self.dep_graph.add_edge(qname, qglob)

             # 3) structs → function
            for st in func.get('structs', []):
                qstruct = '"{}"'.format(st['name'])
                self.dep_graph.add_node(qstruct)
                self.dep_graph.add_edge(qname, qstruct)

            
        # We only want to translate functions that are reachable from main
        reachable_q = nx.descendants(self.dep_graph, '"main_0"') | {'"main_0"'}
        subgraph   = self.dep_graph.subgraph(reachable_q)

        self._dump_graph(self.dep_graph, title="Full Dependency Graph")



        components = nx.weakly_connected_components(subgraph)
        assert len(list(components)) == 1

        try:
            func_ordering = list(reversed(list(nx.topological_sort(subgraph))))
        except nx.NetworkXUnfeasible:
            func_ordering = list(nx.dfs_postorder_nodes(subgraph, source='"main_0"'))
        
        func_ordering = [f.strip('"') for f in func_ordering]


        for func_name in func_ordering:
            funcs = [f for f in static_analysis_results['functions'] if f['name'] == func_name]
            if len(funcs) == 0:
                continue
            func = funcs[0]
            if instrumentation_results is not None:
                instrumentation_logs = [log for log in instrumentation_results if log['name'] == func_name]
                if len(instrumentation_logs) == 0:
                    # Include only covered functions
                    continue
                func['instrumentation'] = {'args': instrumentation_logs[0]['args'],
                                        'return': instrumentation_logs[0]['return']}
            yield func


    

    def _dump_graph(self, graph, title="Dependency Graph"):
        """
        Dump all nodes and edges (with attributes) of a NetworkX graph.
        """
        print(f"\n{title}:")
        print("  Nodes:")
        for n in graph.nodes():
            print(f"    {n}")
        print("  Edges:")
        for u, v, data in graph.edges(data=True):
            # data is a dict of edge‐attributes, e.g. passed_structs
            attrs = ", ".join(f"{k}={v!r}" for k, v in data.items())
            print(f"    {u} -> {v}" + (f"  [{attrs}]" if attrs else ""))
        print("-" * 40)