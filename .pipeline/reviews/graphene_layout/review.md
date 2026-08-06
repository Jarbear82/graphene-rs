# Comprehensive Code Review: `graphene_layout`

## Review: `crates/graphene_layout`

### 1. Algorithm & Modularity
* **Efficiency**: `O(n log n)` — Force calculations utilize `Quadtree` spatial partitioning for Barnes-Hut repulsion and MTV overlap resolution, reducing baseline `O(n²)` spring-embedder complexity to `O(n log n)` for graphs with $N > 100$. Spectral placement landmarking and barycentric ordering optimize initialization phases.
* **Maintainability**: `Straightforward` — Trait-driven design (`Layout<S>`, `IterativeLayout<S>`, `PhaseSteppableLayout<S>`) paired with builder-pattern configuration methods (`with_*`), explicit ownership semantics, and clear intent-revealing module organization keeps cognitive load low.
* **Cohesion**: `Strong` — Layout algorithms are tightly grouped into dedicated submodules by mathematical paradigm (`basic`, `bipartite`, `circular_advanced`, `collision`, `compound`, `cose`, `fcose`, `force`, `force_atlas2`, `fruchterman_reingold`, `grid_sorted`, `hierarchical`, `multilevel`, `planar_shift`, `spectral`, `tree`, `tutte`).
* **Coupling**: `Encapsulated` — Component interaction is strictly governed by `GraphState<S>` via narrow flat array accessors (`state.positions`, `state.sizes`, `state.hierarchy`) and SlotMap keys (`NodeId`, `EdgeId`) rather than broad struct dependencies.

### 2. Encapsulation & System
* **Fidelity**: `Complete` — Fully preserves physical node dimensions (`Size2`), AABB minimum translation vector (MTV) collision separation, multi-level compound parent bounds auto-resizing, and rectangular node boundary edge clipping.
* **Robustness**: `Proven` — Comprehensive edge-case handling for 0-node and 1-node graphs, finite float assertions (`is_finite()`), non-zero distance clamping (`max(0.01)` / `max(1e-4)`), and zero panics across all public APIs.
* **Abstraction**: `Opaque` — Complex numerical physics, Barnes-Hut quadtrees, matrix power iteration, and constraint projection solvers are cleanly hidden behind high-level layout traits and the background `GraphEngineHandle` thread.
* **Adaptability**: `Enabling` — Generic layout composition adapters (`CompoundLayout<L>`, `HierarchicalLayout<L>`, `HybridCompoundLayout<L, P>`, `MultilevelLayout<L>`, `DisconnectedPacker<L>`, `RegionalPartitionLayout<F, L>`) enable flexible component composition without modifying existing files.
* **Alignment**: `Complete` — Domain types (`GraphState`, `Vec2`, `Size2`, `NodeId`, `EdgeId`, `RenderSnapshot`, `LayoutCommand`) accurately map to the user's mental model and data access patterns.
* **Redundancy**: `Minor` — Minor code duplication exists across layout modules for inline LCG pseudo-random number generator closures (`state_lcg.wrapping_mul(...).wrapping_add(...)`) and leaf-descendant recursion helpers.

### 3. File Constraints
* **Length**: `FAIL` (3 files exceed the 1000 lines maximum limit)
  - `src/lib.rs`: `Pass` (66/1000 lines)
  - `src/basic.rs`: `Pass` (455/1000 lines)
  - `src/bipartite.rs`: `Pass` (74/1000 lines)
  - `src/circular_advanced.rs`: `Pass` (166/1000 lines)
  - `src/collision.rs`: `Pass` (492/1000 lines)
  - `src/compound.rs`: `Pass` (418/1000 lines)
  - `src/compound_forces.rs`: `Pass` (63/1000 lines)
  - `src/cose.rs`: `Pass` (324/1000 lines)
  - `src/engine.rs`: `Pass` (728/1000 lines)
  - `src/fcose.rs`: `FAIL` (1231/1000 lines — split required)
  - `src/force.rs`: `Pass` (109/1000 lines)
  - `src/force_atlas2.rs`: `FAIL` (1022/1000 lines — split required)
  - `src/fruchterman_reingold.rs`: `Pass` (125/1000 lines)
  - `src/geometry.rs`: `Pass` (192/1000 lines)
  - `src/grid_sorted.rs`: `Pass` (90/1000 lines)
  - `src/hierarchical.rs`: `Pass` (332/1000 lines)
  - `src/livesim.rs`: `Pass` (532/1000 lines)
  - `src/multigraph.rs`: `Pass` (51/1000 lines)
  - `src/multilevel.rs`: `Pass` (138/1000 lines)
  - `src/packers.rs`: `Pass` (112/1000 lines)
  - `src/pipeline.rs`: `Pass` (232/1000 lines)
  - `src/planar_shift.rs`: `Pass` (142/1000 lines)
  - `src/quadtree.rs`: `Pass` (187/1000 lines)
  - `src/spectral.rs`: `Pass` (368/1000 lines)
  - `src/traits.rs`: `Pass` (227/1000 lines)
  - `src/tree.rs`: `Pass` (165/1000 lines)
  - `src/tutte.rs`: `Pass` (126/1000 lines)
  - `tests/cytoscape_layout_geometry_tests.rs`: `Pass` (96/1000 lines)
  - `tests/graph_type_tests.rs`: `FAIL` (1289/1000 lines — split required)
  - `tests/hierarchical_compound_tests.rs`: `Pass` (109/1000 lines)
  - `tests/layout_configurability_tests.rs`: `Pass` (254/1000 lines)
  - `tests/physical_body_overlap_matrix_tests.rs`: `Pass` (256/1000 lines)
  - `tests/physical_body_overlap_tests.rs`: `Pass` (89/1000 lines)
  - `tests/provenance_check.rs`: `Pass` (55/1000 lines)

### 4. Checklist Audits

#### L1 — Algorithm Design
- [x] **Intent-revealing names**: Variable names like `ideal_edge_length`, `quadtree`, `displacements_x`, `effective_directional_radius`, and `size_aware_ideal_length` clearly convey intent.
- [x] **WHY comments**: Academic citations (`Reference: Dogrusoz et al.`, `Reference: Jacomy et al.`, `Reference: Sugiyama et al.`) and rationale comments explain mathematical design decisions. Every public layout struct is validated for a `Reference:` doc comment by `provenance_check.rs`.
- [x] **Linear execution paths**: Functions consistently use guard clauses and early returns for base cases (`if n == 0 { return; }`).
- [x] **Named boolean expressions**: Boolean factors in force loops and constraint solvers are assigned to clear named variables (`all_zero`, `use_quadtree`, `is_parent_child`).

#### L2 — Modularization Design
- [x] **Single responsibility**: Each submodule focuses on a distinct layout algorithm or utility concern.
- [ ] **Zero code duplication**: Minor duplication of inline LCG pseudo-random number generator closures across layout modules (`basic.rs`, `collision.rs`, `cose.rs`, `force_atlas2.rs`, `pipeline.rs`). Extracting this into a central helper will improve DRY compliance.
- [x] **Minimized inputs**: Functions pass minimal slice arguments and primitive keys (`NodeId`, `EdgeId`, `Vec2`, `Size2`).

#### L3 — Encapsulation Design
- [x] **Pure data structs**: Structs (`FCoseConstraints`, `RenderSnapshot`, `FA2Settings`, `FA2Node`) represent pure data layouts.
- [x] **Private fields by default**: Struct fields remain private unless exposed as explicit configuration fields or DTO properties.
- [x] **Invariants validated in constructors**: Builder pattern methods (`with_*`) validate and construct layout configurations safely.
- [x] **Default/new constructors**: Every layout struct provides `Default::default()` implementations.

#### L4 — Module/Type Relation Design
- [x] **Composition over inheritance**: Generic layout wrappers (`CompoundLayout<L>`, `HierarchicalLayout<L>`, `HybridCompoundLayout<L, P>`, `MultilevelLayout<L>`, `DisconnectedPacker<L>`, `RegionalPartitionLayout<F, L>`) compose inner layouts cleanly.
- [x] **Minimal trait bounds**: Functions constrain types strictly to required traits (`S: Copy + Default`, `L: Layout<S>`).
- [x] **Invariants enforced by type system**: State transitions and execution phases use strongly-typed enums (`CosePhase`, `FCosePhase`, `SugiyamaPhase`, `EngineWorkerState`).

#### L5 — Component & System Design
- [x] **Swappable components**: Algorithms are swappable behind `Layout<S>`, `IterativeLayout<S>`, and `PhaseSteppableLayout<S>`.
- [x] **Narrow interfaces**: Internal quadtree and numerical helper routines remain private/internal; public API exposes clean handles.
- [x] **Backward compatibility**: Builder methods use additive options without breaking changes.
- [x] **Logic decoupled from state**: Layout algorithms operate as pure state transformations on `GraphState<S>`.

### ACTION REQUIRED:
[X] REFACTOR: Split `src/fcose.rs` (1231 lines), `src/force_atlas2.rs` (1022 lines), and `tests/graph_type_tests.rs` (1289 lines) into modular sub-directories with `mod.rs` exposing the public API to satisfy the 1000-line maximum limit per REVIEWING.md §4. Additionally, extract common LCG pseudo-random number generation into a shared internal utility to eliminate minor code duplication across layout modules.
