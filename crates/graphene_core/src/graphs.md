## Graph Types

## Structural

### Flat Graphs
All nodes and edges exist on one flat plane. No node can contain other nodes and edges.  

### Compound Graphs
Nodes can be compound nodes, meaning they contain sub-graphs within them. Which in turn, can contain sub-graphs of their own.  
- **Subgraph**: A graph formed from a subset of vertices and edges of a larger graph.  
- **Induced Subgraph**: A subset of vertices and all the original edges that connect them.  
- **Spanning Tree**: A subgraph that is a tree and includes all the vertices of the original connected graph.  

### Standard Graphs
Standard edges are inherently binary — each edge is an ordered pair of vertices ($e \in V \times V$) with one source and one target. This is the default form in both pure mathematics and computer science (contrast with hypergraphs below).

### Hypergraphs
Graphs with nodes and N-ary Edges. Can be reified into standard graphs by making the N-ary edge a node with multiple binary edges.  

### Memory Representations
The data structures used to store a graph dictate its memory layout and traversal performance.  
- **Edge List**: A simple array of vertex pairs. Highly cache-friendly and contiguous; ideal for fast iterations over all edges.  
- **Adjacency List**: An array where each index represents a vertex, pointing to a list of its neighbors. The standard for sparse graphs.  
- **Adjacency Matrix**: A 2D grid where a `1` (or weight) indicates an edge between row $i$ and column $j$. Best for dense graphs.  
- **Incidence Matrix**: A matrix where rows represent vertices and columns represent edges. A `1` (or `-1` for directed) indicates the vertex is an endpoint of the edge.  

## Classifications

### Edge Attributes
Properties that describe individual edges:  
- **Directed:** Edges have a specific direction (one-way), representing an asymmetric relationship (e.g., $A \to B$).  
- **Undirected**: Edges have no direction, representing symmetric relationships (e.g., $A - B$).  
- **Weighted**: Edges are assigned numerical values (weights) representing costs, distances, or capacities.  
- **Unweighted**: All edges are treated equally without any associated costs.
- **Mixed**: A graph that contains both directed and undirected edges.  

### Structural Constraints
Classifications based on what edges are allowed within the graph:  
- **Simple**: An undirected graph without self-loops or multiple (parallel) edges between the same two vertices.  
- **Multigraph**: Allows multiple edges between the same pair of vertices, but no self-loops.  
- **Pseudograph**: Allows both self-loops (edges connecting a vertex to itself) and multiple edges.  

### Structure & Completeness
- **Null**: Contains vertices but has no edges at all.  
- **Complete**: A simple graph where every distinct vertex is connected to every other vertex by exactly one edge.  
- **Regular**: Every vertex has the exact same number of connected edges (degree).  
- **Bipartite**: Vertices are divided into two disjoint sets, and edges only connect vertices from different sets, not within the same set.  
- **Complete Bipartite**: A bipartite graph where every vertex in the first set is connected to every vertex in the second set.  
- **Cyclic**: Contains at least one closed loop (cycle).  
- **Acyclic**: Contains no cycles (e.g., Trees).  
- **Sparse**: Number of edges is close to the number of vertices. (Best stored as an Adjacency List).  
- **Dense**: Number of edges is close to the maximum possible number of edges. (Best stored as an Adjacency Matrix).  
- **Trivial**: A graph with exactly one vertex and no edges. (Your Null graph is usually defined as having zero vertices and zero edges, though definitions sometimes vary).  

### Connectivity
- **Connected**: There is a path allowing you to travel from any vertex to any other vertex in the graph.  
- **Disconnected**: At least one pair of vertices has no path connecting them, often splitting the graph into separate "components".  
- **Strongly Connected**: (Directed) There is a valid, directed path from every vertex to every other vertex.  
- **Weakly Connected**: (Directed) The graph would be connected if you ignored the direction of the edges, but lacks directed paths between all pairs.  

### Subgraphs & Coloring Properties
- **Clique**: A subset of vertices where *every* two distinct vertices are adjacent (a complete subgraph).  
- **Independent Set**: A subset of vertices where *no* two vertices are adjacent (an edgeless induced subgraph).  
- **Chromatic Number**: The minimum number of colors needed to color a graph's vertices such that no two adjacent vertices share a color.  

### Layout & Topology
These concepts directly impact visualization, clustering, and geometric algorithms:  
- **Graph Minor**: A graph formed by deleting vertices/edges and, crucially, by **edge contraction** (merging two adjacent vertices into one). This is the mathematical foundation for collapsing nodes in compound graphs.  
- **Crossing Number**: The minimum number of edge intersections required when drawing the graph on a 2D plane. (Planar graphs have a crossing number of 0).  
- **Intersection Graph**: A graph where vertices represent geometric objects (like bounding boxes or circles) and edges represent overlaps/intersections. Extremely useful for collision detection in rendering.  

### Structural Equivalence
- **Isomorphic Graphs**: Two graphs that contain the exact same number of vertices and edges connected in the same way, even if drawn differently or labeled differently — they share structural equivalence.  

## Special Graph Types
Named graph structures with distinctive patterns:  
- **Tree**: A connected, undirected graph with no cycles.  
- **Planar**: Can be drawn on a 2D plane without any of its edges crossing over one another.  
- **Star**: Consists of one central vertex connected to all other vertices, resembling a star shape.  
- **Directed Acyclic (DAG)**: A directed graph with no directed cycles. Absolutely essential for dependency resolution, build systems (like Make), and version control (like Git).  
- **Forest**: An undirected graph with no cycles, essentially a disjoint collection of Trees.  
- **Path**: A graph where all vertices can be listed in the order of a single path.  
- **Cycle**: A graph that consists of a single, unbroken cycle of vertices (a ring).  
- **Eulerian Graph**: A graph containing an Eulerian circuit — a trail that visits every *edge* exactly once and starts and ends on the same vertex.  
- **Hamiltonian Graph**: A graph containing a Hamiltonian cycle — a path that visits every *vertex* exactly once and returns to the start.  
- **Tournament**: A directed graph obtained by assigning a direction to each edge of an undirected complete graph (every node has a one-way connection to every other node).  

## Network Science Classifications
Graphs classified by their degree distribution and emergent topology, particularly relevant in network science:  
- **Scale-Free Graphs**: Degree distribution follows a power law — a few nodes act as massive hubs while most have few connections (e.g., the internet, social networks).  
- **Small-World Graphs**: Feature short average path lengths between any two vertices despite high clustering — "six degrees of separation" behavior.  

## Computational Graph Models
Graph structures defined by their use in data modeling and computation:  
- **Property Graphs**: Directed multigraphs where both nodes and edges can hold arbitrary key-value properties. Highly relevant in graph databases (e.g., Neo4j).  

## Matrix Algebra / Spectral Properties
Force-directed and spectral layout algorithms rely on these derived mathematical structures:  
- **Degree Matrix**: A diagonal matrix containing the degree (number of edges) of each vertex.  
- **Laplacian Matrix**: Defined mathematically as $L = D - A$ (where $D$ is the Degree matrix and $A$ is the Adjacency matrix). Its eigenvalues are often used to calculate optimal 2D/3D layouts.  
- **Distance Matrix**: A matrix containing the shortest path lengths between all pairs of vertices, rather than just immediate adjacencies.
