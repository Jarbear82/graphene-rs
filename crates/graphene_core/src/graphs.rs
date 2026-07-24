// ## Structural

// ### Flat Graphs
// All nodes and edges exist on one flat plane. No node can contain other nodes and edges.

// ### Compound Graphs
// Nodes can be compound nodes, meaning they contain sub-graphs within them. Which in turn, can contain sub-graphs of their own.

// ### Hypergraphs
// Can be reified into binary graphs by makeing the n-nary edge a node with multiple binary edges.

// ## Classifications

// ### Edges
// Directed: Edges have a specific direction (one-way), representing an asymmetric relationship (e.g., \(A \to B\)).
// Undirected: Edges have no direction, representing symmetric relationships (e.g., \(A - B\)).
// Weighted: Edges are assigned numerical values (weights) representing costs, distances, or capacities.
// Unweighted: All edges are treated equally without any associated costs.
// Simple: An undirected graph without self-loops or multiple (parallel) edges between the same two vertices.
// Multigraph: Allows multiple edges between the same pair of vertices, but no self-loops.
// Pseudograph: Allows both self-loops (edges connecting a vertex to itself) and multiple edges.

// ### Structure & Completenss
// Null Graph: Contains vertices but has no edges at all.
// Complete Graph: A simple graph where every distinct vertex is connected to every other vertex by exactly one edge.
// Regular Graph: Every vertex has the exact same number of connected edges (degree).
// Bipartite Graph: Vertices are divided into two disjoint sets, and edges only connect vertices from different sets, not within the same set.
// Cyclic Graph: Contains at least one closed loop (cycle).
// Acyclic Graph: Contains no cycles (e.g., Trees).

// ### Connectivity
// Connected Graph: There is a path allowing you to travel from any vertex to any other vertex in the graph.
// Disconnected Graph: At least one pair of vertices has no path connecting them, often splitting the graph into separate "components".

// ### Special
// Tree: A connected, undirected graph with no cycles.
// Planar Graph: Can be drawn on a 2D plane without any of its edges crossing over one another.
// Star Graph: Consists of one central vertex connected to all other vertices, resembling a star shape.
