## Review: crates/graphene_gpui

### 1. Algorithm & Modularity
* **Efficiency**: `O(n)` — Edge removal bug in `GraphView::apply_update` executes `self.edge_order.retain(|&x| x != x)` which wipes all edge ordering in O(E) time; spatial grid hit testing falls back to full O(N) node iteration when spatial grid cells are empty; `GraphCanvas::into_element` performs repeated O(depth) parent traversals (`get_visible_rep`) for every edge and node.
* **Maintainability**: `Deducible` — View and interaction state structures decouple rendering from graph state updates, but obsolete exported stubs (`update_node_shape`, `update_edge_width`), unused fields (`is_box_selecting`, `box_select_rect`), and dead code in `draw_pipeline.rs` clutter the public API.
* **Cohesion**: `Partial` — `GraphCanvas::into_element` is a monolith handling edge path calculation, node element creation, label layout, canvas grid drawing, and UI badge rendering. Color conversion bitwise math is duplicated across `style_bridge.rs` and `graph_canvas.rs`.
* **Coupling**: `Encapsulated` — `graphene_gpui` integrates cleanly with `graphene_core`, `graphene_style`, and `graphene_layout` via narrow extension traits (`Vec2Ext`, `Size2Ext`) and DTO view types (`NodeViewData`, `EdgeViewData`).

### 2. Encapsulation & System
* **Fidelity**: `Complete` — `GraphView` faithfully models `GraphState` topology, positions, sizes, and parent-child hierarchy relations.
* **Robustness**: `Fragile` — Critical bug in `GraphUpdate::EdgeRemoved` (`self.edge_order.retain(|&x| x != x)`) completely wipes `edge_order` when removing a single edge. Compiler warnings for unused mutability and unused variables indicate untested edge cases.
* **Abstraction**: `Opaque` — Hides low-level GPUI drawing and event coordinates behind high-level `GraphCanvas` and `Viewport` abstractions.
* **Adaptability**: `Enabling` — Immediate mode render pipeline and configurable `CanvasConfig` enable extension to custom rendering modes and layouts without altering existing types.

### 3. File Constraints
* **Length**: `Pass` (All files under 1000 lines limit: `graph_canvas.rs` at 672 lines, `view.rs` at 393 lines, `state.rs` at 330 lines, `draw_pipeline.rs` at 309 lines, `style_bridge.rs` at 80 lines, `convert.rs` at 45 lines, `lib.rs` at 11 lines).

### 4. Checklist Audits

#### L1 — Algorithm Design
- [x] Names are intent-revealing: Typos present (`x != x` instead of `x != id`), unused variables (`nodes_count`, `label_instances`).
- [ ] Comments explain WHY, not WHAT: Misleading comment `// retain other ids` above `self.edge_order.retain(|&x| x != x);` which actually deletes all edge IDs.
- [x] Guard clauses used to keep execution paths linear in `Viewport::fit_to_graph` and `GraphCanvas::into_element`.
- [x] Complex boolean checks in selection and hit-testing logic broken down into named variables (`is_primary`, `is_secondary`, `is_neighbor`, `is_faded`).

#### L2 — Modularization Design
- [ ] Single responsibility per function: `GraphCanvas::into_element` (~450 lines) violates single responsibility by combining edge path generation, node element construction, label text measurement/truncation, canvas grid painting, and UI overlay rendering into a single method.
- [ ] Zero code duplication: Bitwise RGBA color conversion logic is duplicated across `style_bridge::color_value_to_rgba` and `graph_canvas::color_to_gpui`.
- [x] Inputs minimized: Functions pass explicit slices and value types where possible, but `GraphCanvas` struct accepts 14 individual parameters in its constructor.

#### L3 — Encapsulation Design
- [x] Structs are pure data layouts: `NodeViewData`, `EdgeViewData`, `Viewport`, `CanvasConfig`.
- [x] Struct fields private by default except DTOs and public configuration structs.
- [x] Invariants validated in constructors and `Default` implementations (`CanvasConfig::default()`, `Viewport::new()`).
- [x] Standard constructors provided (`new`, `default`, `from_state`).

#### L4 — Module/Type Relation Design
- [x] Composition favored over trait hierarchies.
- [x] Trait bounds kept minimal (`S: Copy + Send + 'static`).
- [x] Invariants enforced by the type system via `GraphView` state synchronization.

#### L5 — Component & System Design
- [x] Components swappable behind traits (`Vec2Ext`, `GpuiPointExt`, `Size2Ext`, `GpuiSizeExt`).
- [ ] Interfaces are narrow: Public API in `lib.rs` exports obsolete empty stubs (`update_node_shape`, `update_edge_width`) and unused fields (`is_box_selecting`, `box_select_rect` on `InteractionState`).
- [x] Backward compatibility preserved through additive configuration options in `CanvasConfig`.
- [x] Logic decoupled from state: Rendering commands and canvas elements operate on `GraphView` and `Viewport` without mutating underlying graph state.

### ACTION REQUIRED:
[X] REFACTOR:
1. Fix critical edge removal bug in `crates/graphene_gpui/src/view.rs` line 296-297: remove redundant `self.edge_order.retain(|&x| x != x);` which empties `edge_order` on any edge removal.
2. Remove unused empty function stubs `update_node_shape` and `update_edge_width` in `crates/graphene_gpui/src/interaction/state.rs` and update exports in `src/lib.rs`.
3. Clean up compiler warnings in `crates/graphene_gpui/src/render/draw_pipeline.rs` (remove `mut` from `label_instances`) and `crates/graphene_gpui/src/render/graph_canvas.rs` (remove unused `nodes_count`).
4. Deduplicate color conversion logic by re-using `color_value_to_rgba` from `style_bridge.rs` in `color_to_gpui` within `graph_canvas.rs`.
5. Remove or utilize dead fields (`is_box_selecting`, `box_select_rect`) in `InteractionState`.
