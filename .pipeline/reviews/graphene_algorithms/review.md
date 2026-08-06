## Review: crates/graphene_algorithms

### 1. Algorithm & Modularity
* **Efficiency**: `O(n log n)` — High core efficiency via CSR `EdgeTopology` and flat `GraphState` vectors for primary graph operations, though temporary `GraphState` allocations and string-to-NodeId HashMap lookups in wrapper layers add minor overhead.
* **Maintainability**: `Adjustable` — Memory ownership and lifetimes are clean and borrow-checker compliant; however, parallel graph abstractions (custom `Graph` structs vs `GraphState` vs custom `Graph` traits) increase maintenance burden.
* **Cohesion**: `Partial` — Module boundaries (`centrality`, `clustering`, `pathfinding`, `search_traversal`) cleanly separate domain concerns, but algorithmic logic is split between core `graph_state_*` implementations and duplicate wrapper modules.
* **Coupling**: `Complex` — Multiple competing graph data representations couple algorithm implementations to redundant translation layers and standalone re-implementations.

### 2. Encapsulation & System
* **Fidelity**: `Complete` — Algorithmic logic (Brandes, PageRank, Floyd-Warshall, Bellman-Ford, A*, Kruskal, Tarjan SCC, Hopcroft-Tarjan, K-Means/Medoids/FCM, Hierholzer, Karger-Stein) faithfully represents graph structural algorithms and data transformations.
* **Robustness**: `Strong` — Proper use of `Result`/`Option` error returns (`ClusteringError`, `Option<Vec<NodeId>>`, `AStarResult`), boundary guards for empty graph inputs, and zero `.unwrap()` on public I/O.
* **Abstraction**: `Porous` — Public module surface exposes redundant graph data structures (`GraphState`, custom `Graph` structs with `String`/`u64`, custom `Graph` traits) instead of a unified trait/adapter interface.
* **Adaptability**: `Straightforward` — New algorithm additions are easy to integrate, but extending existing features requires updating parallel wrapper types and duplicate logic.

### 3. File Constraints
* **Length**: `Pass` (All 32 source files <= 522 lines; maximum limit is 1000 lines)

### 4. Checklist Audits

#### L1 — Algorithm Design
- [x] **Intent-revealing names**: Variable names across algorithms (`start_node`, `edge_weight`, `dampening_factor`, `damping`, `cut_vertices`, `visited`, `distances`) clearly reveal intent.
- [ ] **Comments explain WHY, not WHAT**: Several complex routines (e.g. `canonical_ordering.rs::compute_canonical_ordering`, `planarity.rs::has_k5_or_k33_subgraph`, `karger_stein.rs::collapse`) lack WHY comments explaining algorithmic invariants or heuristic rationale.
- [ ] **Paths are linear — guard clauses and early returns, no deep nesting**: Deep nesting (4+ levels) present in `canonical_ordering.rs`, `planarity.rs`, `hierarchical_clustering.rs`, and `floyd_warshall.rs`.
- [x] **Boolean chains broken into named variables**: Complex conditionals in search and clustering algorithms break intermediate predicates into named booleans (e.g., `is_forward`, `should_update`, `consistent`).

#### L2 — Modularization Design
- [x] **Each function has exactly one responsibility**: Core traversal and metric algorithms are broken into focused helper routines (`build_index`, `strongconnect`, `contract_until`, `refine_exemplars`).
- [ ] **Zero code duplication — all repeated patterns extracted**: Significant code duplication exists between `centrality/closeness_centrality.rs` (which re-implements Floyd-Warshall and Dijkstra) and `pathfinding/floyd_warshall.rs` / `pathfinding/dijkstra.rs` / `graph_state_pathfinding.rs`. Additionally, repeated GraphState translation boilerplate exists across `centrality/*`, `pathfinding/*`, and `search_traversal/*`.
- [x] **Inputs minimized — only required data passed**: Closures and slices (`&[NodeId]`, `&[Edge]`, `Fn(EdgeId) -> f32`) are used effectively to limit parameter footprint.

#### L3 — Encapsulation Design
- [x] **Struct names are nouns representing pure data layouts**: Structs (`GraphState`, `CsrMatrix`, `AStarResult`, `MinCutResult`, `EulerResult`, `BiconnectedResult`) represent pure data layouts.
- [ ] **Fields are private by default; public only for DTOs and config**: Most structs use public fields appropriate for DTOs/configs, but options structs (`HierarchicalClusteringOptions`, `CentralityOptions`, `PageRankConfig`, `ApConfig`, `ClusterOptions`) lack constructor validation enforcement.
- [ ] **Invariants validated in constructors (`new` / `try_new`)**: Configurations like `ApConfig` have explicit `.validate()` methods, but other option structs rely on caller validation.
- [x] **Constructors always provided (`Default::default()`, `Self::new()`, or `Self::try_new()`)**: `Default` implementations are provided for configuration types.

#### L4 — Module/Type Relation Design
- [x] **Composition favored over inheritance**: Struct fields and generics are used instead of deep trait hierarchies.
- [x] **Trait bounds are minimal**: Generic parameter bounds (`N: Clone + Eq + Hash`, `S: Copy`) are limited strictly to what functions call.
- [ ] **Invariants enforced by the type system, not caller discipline**: Node identifiers vary between `NodeId` (`graphene_core`), custom `NodeId(u64)`, `usize`, and `String` across different modules, requiring callers to adapt types depending on the module.

#### L5 — Component & System Design
- [ ] **Components are swappable behind traits**: Multiple algorithms define isolated graph traits or structs instead of implementing a unified graph adapter or trait across `graphene_algorithms`.
- [ ] **Interfaces are narrow (`pub(crate)` visibility, minimal `pub` surface)**: Duplicate helper structs and parallel graph types are exposed with `pub` visibility in submodule root files.
- [x] **Backward compatibility considered**: Public API exports are grouped in `src/lib.rs` for convenience.
- [ ] **Logic decoupled from state**: Inconsistent design paradigms — some modules expose methods on custom `Graph` structs (`degree_centrality.rs`, `page_rank.rs`, `bfs_dfs.rs`, `hopcroft_tarjan_biconnected.rs`), while others expose standalone functions operating on `&GraphState` (`graph_state_metrics.rs`, `graph_state_pathfinding.rs`).

### ACTION REQUIRED:
[X] REFACTOR:
1. **Unify Graph Representations & Eliminate Duplicate Implementations**:
   - In `centrality/closeness_centrality.rs`: Remove duplicate standalone re-implementations of `floyd_warshall` and `dijkstra_shortest_paths`, and remove duplicate `NodeId` definition. Delegate shortest path calculations to `pathfinding/floyd_warshall.rs` and `pathfinding/dijkstra.rs` or `graph_state_metrics`.
   - Fix typo in `centrality/closeness_centrality.rs`: Rename `closeness_centralty_one_node` to `closeness_centrality_one_node`.
2. **Flatten Deep Nesting & Add WHY Comments**:
   - Refactor multi-nested loops in `canonical_ordering.rs`, `planarity.rs` (`has_k5_or_k33_subgraph`), `hierarchical_clustering.rs`, and `floyd_warshall.rs` using guard clauses and early returns.
   - Add explicit WHY comments explaining algorithmic invariants in `canonical_ordering.rs`, `planarity.rs`, and `karger_stein.rs`.
3. **Consolidate Conversion Boilerplate**:
   - Extract a common helper or adapter for building temporary `GraphState` instances from generic node/edge collections to eliminate duplicated setup logic across `betweenness_centrality.rs`, `page_rank.rs`, `dijkstra.rs`, `kruskal.rs`, `floyd_warshall.rs`, and `tarjan_strongly_connected.rs`.
