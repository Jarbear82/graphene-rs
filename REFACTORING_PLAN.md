# Graphene Library Refactoring Plan

## Executive Summary

The `interactive_demo` application contains domain-specific logic that should be migrated to the core `graphene_*` crates. This refactoring will improve code organization, reduce duplication, and enable future consumers to build interactive tools without reimplementing complex graph operations.

---

## Current State Analysis

### Location of Domain Logic

| Component | File(s) | Functionality |
|-----------|---------|---------------|
| **Physics** | `app_physics.rs` | Live simulation with Barnes-Hut option, collision resolution |
| **Hierarchy Queries** | `app_physics.rs`, `render.rs` | Ancestor/descendant checks, leaf traversal |
| **Hit Testing** | `theme.rs` | Point-to-segment distance for edge interaction |
| **Edge Rendering** | `graph_canvas.rs` | Bezier curves, taxi routing, label positioning |

### Why This is Problematic

1. **Physics**: Duplicates `ForceDirectedLayout` logic with manual force calculations
2. **Hierarchy Queries**: Core graph operations not exposed via public API
3. **Hit Testing**: GPUI-specific math duplicated in multiple places
4. **Edge Rendering**: Path building logic mixed with application code

---

## Proposed Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    interactive_demo (app)                    │
│  ┌──────────┐  ┌────────────┐  ┌────────────────────────┐   │
│  │ Physics  │  │ Hierarchy  │  │  Geometry & Rendering  │   │
│  │  API     │  │  Traversal │  │    Helper Functions    │   │
│  └────┬─────┘  └──────┬─────┘  └──────────┬─────────────┘   │
└───────┼──────────────┴─────────────────────┼────────────────┘
        │                                    │
┌───────┴────────────────────────────────────┴────────────────┐
│              graphene_* Core Crates                        │
│  ┌──────────────────┐  ┌──────────────────────┐            │
│  │ graphene_core    │  │ graphene_layout      │            │
│  │ - Hierarchy API  │  │ - Live Simulation    │            │
│  │ - Math Helpers   │  │ - Geometry Utilities │            │
│  └────────┬─────────┘  └──────────┬───────────┘            │
└───────────┼───────────────────────┼─────────────────────────┘
            │                       │
┌───────────┴───────────────────────┴─────────────────────────┐
│                    graphene_gpui (rendering)               │
│  ┌──────────────────┐  ┌────────────────────────────────┐   │
│  │ Path Building    │  │ Edge Rendering Helpers         │   │
│  │ (PathBuilder)    │  │ (bezier, taxi, arrowheads)     │   │
│  └──────────────────┘  └─────────────────────────────────┘   │
└──────────────────────────────────────────────────────────────┘
```

---

## Implementation Plan

### Phase 1: Hierarchy Traversal API (`graphene_core`)

**Goal**: Add public hierarchy traversal methods to `GraphState` or new `Hierarchy` struct.

#### New Methods in `graphene_core/src/state/mod.rs`

```rust
// After line 46 (after GraphState impl start)
impl<S> GraphState<S> {
    // Existing methods...
    
    // === NEW HIERARCHY TRAVERSAL METHODS ===
    
    /// Check if `child_idx` is a descendant of `parent_idx`
    pub fn is_ancestor(&self, child_idx: usize, parent_idx: usize) -> bool
    
    /// Get all leaf descendants of a node (nodes with no children)
    pub fn get_leaf_descendants(&self, node_idx: usize) -> Vec<usize>
    
    /// Check if a node has any children
    pub fn is_parent(&self, idx: usize) -> bool
    
    /// Get the visible representative for a node in collapsed hierarchy
    /// Returns the first ancestor that is not collapsed (or self if none are collapsed)
    pub fn get_visible_representative(
        &self,
        node_id: NodeId,
        collapsed_parents: &HashSet<NodeId>,
    ) -> NodeId
    
    /// Calculate nesting depth of a node in the hierarchy
    pub fn get_nesting_depth(&self, node_id: NodeId) -> usize
    
    /// Get all descendants of a node (children, grandchildren, etc.)
    pub fn get_all_descendants(&self, node_idx: usize) -> Vec<usize>
}
```

#### Implementation Details

```rust
// In state/mod.rs after GraphState impl start

/// Check if `child_idx` is a descendant of `parent_idx`
pub fn is_ancestor(&self, child_idx: usize, parent_idx: usize) -> bool {
    let parent_id = self.node_index_to_id[parent_idx];
    let mut curr_idx = child_idx;
    
    while let Some(&parent_id_opt) = self.hierarchy.parent.get(curr_idx) {
        if let Some(parent_node_id) = parent_id_opt {
            if parent_node_id == parent_id {
                return true;
            }
            if let Some(&next_idx) = self.node_keys.get(parent_node_id) {
                curr_idx = next_idx;
            } else {
                break;
            }
        } else {
            break;
        }
    }
    false
}

/// Get all leaf descendants of a node (nodes with no children)
pub fn get_leaf_descendants(&self, node_idx: usize) -> Vec<usize> {
    let mut leaves = Vec::new();
    
    if self.hierarchy.first_child.get(node_idx).is_none() {
        // Node has no children, it's already a leaf
        leaves.push(node_idx);
        return leaves;
    }
    
    let mut stack = vec![node_idx];
    while let Some(curr) = stack.pop() {
        if self.hierarchy.first_child.get(curr).is_none() {
            // Leaf node
            leaves.push(curr);
        } else {
            // Traverse children
            let mut next_child = *self.hierarchy.first_child.get(curr);
            while let Some(child_id) = next_child {
                if let Some(&child_idx) = self.node_keys.get(child_id) {
                    stack.push(child_idx);
                    next_child = *self.hierarchy.next_sibling.get(child_idx);
                } else {
                    break;
                }
            }
        }
    }
    
    leaves
}

/// Check if a node has any children
pub fn is_parent(&self, idx: usize) -> bool {
    self.hierarchy.first_child.get(idx).is_some()
}

/// Calculate nesting depth of a node in the hierarchy
pub fn get_nesting_depth(&self, mut node_id: NodeId) -> usize {
    let mut depth = 0;
    
    while let Some(&idx) = self.node_keys.get(node_id) {
        if let Some(parent_id) = *self.hierarchy.parent.get(idx) {
            node_id = parent_id;
            depth += 1;
        } else {
            break;
        }
    }
    
    depth
}

/// Get the visible representative for a node in collapsed hierarchy
pub fn get_visible_representative(
    &self,
    mut node_id: NodeId,
    collapsed_parents: &HashSet<NodeId>,
) -> NodeId {
    let mut rep = node_id;
    
    while let Some(&idx) = self.node_keys.get(node_id) {
        if let Some(parent_id) = *self.hierarchy.parent.get(idx) {
            if collapsed_parents.contains(&parent_id) {
                rep = parent_id;  // Use collapsed parent as representative
            }
            node_id = parent_id;
        } else {
            break;
        }
    }
    
    rep
}

/// Get all descendants of a node (children, grandchildren, etc.)
pub fn get_all_descendants(&self, node_idx: usize) -> Vec<usize> {
    let mut descendants = Vec::new();
    let mut stack = vec![node_idx];
    
    while let Some(curr) = stack.pop() {
        // Get first child
        if let Some(child_id) = *self.hierarchy.first_child.get(curr) {
            if let Some(&child_idx) = self.node_keys.get(child_id) {
                descendants.push(child_idx);
                stack.push(child_idx);
                
                // Traverse siblings
                let mut next_sibling = *self.hierarchy.next_sibling.get(child_idx);
                while let Some(sib_id) = next_sibling {
                    if let Some(&sib_idx) = self.node_keys.get(sib_id) {
                        descendants.push(sib_idx);
                        stack.push(sib_idx);
                    }
                    next_sibling = *self.hierarchy.next_sibling.get(sib_id);
                }
            }
        }
    }
    
    descendants
}
```

#### Tests in `graphene_core/src/state/mod.rs`

```rust
#[cfg(test)]
mod tests {
    // ... existing tests ...
    
    #[test]
    fn test_is_ancestor() {
        let mut state = GraphState::new();
        
        let parent_id = state.add_node(NodeData::default(), ComputedStyle::default());
        let child_id = state.add_node(NodeData::default(), ComputedStyle::default());
        let grandchild_id = state.add_node(NodeData::default(), ComputedStyle::default());
        
        state.reparent_node(child_id, parent_id);
        state.reparent_node(grandchild_id, child_id);
        
        let &parent_idx = state.node_keys.get(parent_id).unwrap();
        let &child_idx = state.node_keys.get(child_id).unwrap();
        let &grandchild_idx = state.node_keys.get(grandchild_id).unwrap();
        
        assert!(state.is_ancestor(child_idx, parent_idx));
        assert!(state.is_ancestor(grandchild_idx, parent_idx));
        assert!(state.is_ancestor(grandchild_idx, child_idx));
        assert!(!state.is_ancestor(parent_idx, child_idx));
    }
    
    #[test]
    fn test_get_leaf_descendants() {
        let mut state = GraphState::new();
        
        let root_id = state.add_node(NodeData::default(), ComputedStyle::default());
        let child1_id = state.add_node(NodeData::default(), ComputedStyle::default());
        let child2_id = state.add_node(NodeData::default(), ComputedStyle::default());
        let grandchild_id = state.add_node(NodeData::default(), ComputedStyle::default());
        
        state.reparent_node(child1_id, root_id);
        state.reparent_node(child2_id, root_id);
        state.reparent_node(grandchild_id, child1_id);
        
        let &root_idx = state.node_keys.get(root_id).unwrap();
        let leaves = state.get_leaf_descendants(root_idx);
        
        assert!(leaves.contains(&state.node_keys.get(child2_id).unwrap().clone()));
        assert!(leaves.contains(&state.node_keys.get(grandchild_id).unwrap().clone()));
    }
    
    #[test]
    fn test_get_visible_representative() {
        let mut state = GraphState::new();
        
        let parent_id = state.add_node(NodeData::default(), ComputedStyle::default());
        let child_id = state.add_node(NodeData::default(), ComputedStyle::default());
        
        state.reparent_node(child_id, parent_id);
        
        let collapsed_parents = HashSet::from([parent_id]);
        
        // Child's representative should be parent (collapsed)
        assert_eq!(
            state.get_visible_representative(child_id, &collapsed_parents),
            parent_id
        );
    }
}
```

---

### Phase 2: Live Physics Simulation (`graphene_layout`)

**Goal**: Create `LiveForceSimulation` struct that can be "ticked" incrementally.

#### New Module in `graphene_layout/src/livesim.rs`

```rust
use crate::quadtree::Quadtree;
use graphene_core::{math::Vec2, GraphState};

/// Incremental force-directed simulation that can be advanced frame-by-frame
pub struct LiveForceSimulation {
    /// Current positions (reference to state)
    positions: Vec<&'static mut Vec2>,
    
    /// Simulation parameters
    pub k_rep: f32,
    pub k_att: f32,
    pub gravity: f32,
    pub ideal_length: f32,
    
    /// Temperature for simulated annealing
    pub temperature: f32,
    pub cooling_rate: f32,
    
    /// Barnes-Hut parameters
    pub use_barnes_hut: bool,
    pub theta: f32,
}

impl LiveForceSimulation {
    /// Create a new simulation from a GraphState
    pub fn new(state: &mut GraphState<impl Copy>) -> Self {
        let n = state.node_index_to_id.len();
        
        // Collect mutable references to positions (unsafe but safe within one frame)
        // In practice, this would use interior mutability or borrowck patterns
        let positions = (0..n)
            .map(|i| {
                // This is a simplification - actual implementation needs unsafe
                state.positions.get_mut(i)
            })
            .collect::<Vec<_>>();
        
        Self {
            positions,
            k_rep: 2500.0,
            k_att: 0.06,
            gravity: 0.3,
            ideal_length: 50.0,
            temperature: 10.0,
            cooling_rate: 0.95,
            use_barnes_hut: true,
            theta: 0.5,
        }
    }
    
    /// Advance the simulation by one step
    pub fn tick(&mut self, state: &mut GraphState<impl Copy>) {
        let n = self.positions.len();
        
        if n == 0 {
            return;
        }
        
        // Compute repulsive forces (with Barnes-Hut option)
        let mut forces = vec![Vec2::default(); n];
        
        for i in 0..n {
            for j in (i + 1)..n {
                let pos_i = *self.positions[i];
                let pos_j = *self.positions[j];
                
                let dx = pos_j.x - pos_i.x;
                let dy = pos_j.y - pos_i.y;
                let dist_sq = dx * dx + dy * dy + 0.01;
                let dist = dist_sq.sqrt();
                
                // Skip if in ancestor relationship
                if self.is_ancestor(state, i, j) {
                    continue;
                }
                
                let force = self.k_rep / dist_sq;
                let fx = -force * dx / dist;
                let fy = -force * dy / dist;
                
                forces[i].x += fx;
                forces[i].y += fy;
                forces[j].x -= fx;
                forces[j].y -= fy;
            }
        }
        
        // Compute attractive spring forces
        for i in 0..state.edges.len() {
            let src = *state.edge_sources.get(i);
            let tgt = *state.edge_targets.get(i);
            
            if let (Some(&src_idx), Some(&tgt_idx)) =
                (state.node_keys.get(src), state.node_keys.get(tgt))
            {
                let pos_src = *self.positions[src_idx];
                let pos_tgt = *self.positions[tgt_idx];
                
                let dx = pos_tgt.x - pos_src.x;
                let dy = pos_tgt.y - pos_src.y;
                let dist_sq = dx * dx + dy * dy + 0.01;
                let dist = dist_sq.sqrt();
                
                let force = self.k_att * (dist - self.ideal_length);
                let fx = (dx / dist) * force;
                let fy = (dy / dist) * force;
                
                forces[src_idx].x += fx;
                forces[src_idx].y += fy;
                forces[tgt_idx].x -= fx;
                forces[tgt_idx].y -= fy;
            }
        }
        
        // Apply forces with temperature limiting
        for i in 0..n {
            let force_len = (forces[i].x * forces[i].x + forces[i].y * forces[i].y + 0.01).sqrt();
            let limit = force_len.min(self.temperature);
            
            let dx = (forces[i].x / force_len) * limit;
            let dy = (forces[i].y / force_len) * limit;
            
            self.positions[i].x += dx;
            self.positions[i].y += dy;
        }
        
        // Apply gravity
        for i in 0..n {
            self.positions[i].x -= self.positions[i].x * self.gravity;
            self.positions[i].y -= self.positions[i].y * self.gravity;
        }
        
        // Cool down
        self.temperature *= self.cooling_rate;
    }
    
    /// Resolve overlapping nodes using collision detection
    pub fn resolve_collisions(&mut self, state: &mut GraphState<impl Copy>, padding: f32) {
        let n = self.positions.len();
        
        if n == 0 {
            return;
        }
        
        // Get sizes from state
        let mut sizes = vec![graphene_core::Size2::new(0.0, 0.0); n];
        for i in 0..n {
            let id = state.node_index_to_id[i];
            if let Some(&idx) = state.node_keys.get(id) {
                sizes[i] = *state.sizes.get(idx);
            }
        }
        
        // Resolve overlaps
        for _ in 0..4 {
            for i in 0..n {
                for j in (i + 1)..n {
                    if self.is_ancestor(state, i, j) || self.is_ancestor(state, j, i) {
                        continue;
                    }
                    
                    let pos_i = *self.positions[i];
                    let pos_j = *self.positions[j];
                    
                    let dx = pos_j.x - pos_i.x;
                    let dy = pos_j.y - pos_i.y;
                    
                    let min_dx = (sizes[i].w + sizes[j].w) / 2.0 + padding;
                    let min_dy = (sizes[i].h + sizes[j].h) / 2.0 + padding;
                    
                    let overlap_x = min_dx - dx.abs();
                    let overlap_y = min_dy - dy.abs();
                    
                    if overlap_x > 0.0 && overlap_y > 0.0 {
                        // ... collision resolution logic (same as app_physics.rs) ...
                    }
                }
            }
        }
    }
    
    /// Check if two nodes are in ancestor-descendant relationship
    fn is_ancestor(&self, state: &GraphState<impl Copy>, child_idx: usize, parent_idx: usize) -> bool {
        // Delegate to GraphState method (will be implemented in Phase 1)
        state.is_ancestor(child_idx, parent_idx)
    }
}
```

#### Integration with `graphene_layout` exports

```rust
// In graphene_layout/src/lib.rs
pub mod livesim;
pub use livesim::LiveForceSimulation;
```

---

### Phase 3: Geometry and Hit Testing Helpers (`graphene_core::math`)

**Goal**: Add geometric utility functions to the math module.

#### New Functions in `graphene_core/src/math.rs`

```rust
// After existing Vec2 implementations

impl Vec2 {
    // ... existing methods ...
    
    /// Compute distance from this point to a line segment
    pub fn distance_to_segment(&self, a: Vec2, b: Vec2) -> f32 {
        let px_val = self.x;
        let py_val = self.y;
        let ax = a.x;
        let ay = a.y;
        let bx = b.x;
        let by = b.y;
        
        let dx = bx - ax;
        let dy = by - ay;
        let len_sq = dx * dx + dy * dy;
        
        if len_sq == 0.0 {
            // Segment is a point
            let rx = px_val - ax;
            let ry = py_val - ay;
            return (rx * rx + ry * ry).sqrt();
        }
        
        // Project point onto line segment [0,1]
        let t = ((px_val - ax) * dx + (py_val - ay) * dy) / len_sq;
        let t = t.clamp(0.0, 1.0);
        
        let proj_x = ax + t * dx;
        let proj_y = ay + t * dy;
        
        // Distance from point to projection
        let rx = px_val - proj_x;
        let ry = py_val - proj_y;
        (rx * rx + ry * ry).sqrt()
    }
    
    /// Compute perpendicular offset vector
    pub fn perpendicular(&self) -> Self {
        Vec2::new(-self.y, self.x)
    }
    
    /// Project this vector onto another vector
    pub fn project_onto(self, other: Self) -> Self {
        let dot = self.x * other.x + self.y * other.y;
        let len_sq = other.x * other.x + other.y * other.y;
        
        if len_sq == 0.0 {
            return Self::default();
        }
        
        let scale = dot / len_sq;
        Vec2::new(other.x * scale, other.y * scale)
    }
}

// After existing Size2 implementations

impl Size2 {
    /// Compute the corners of a rectangle centered at origin
    pub fn corners(&self) -> [Vec2; 4] {
        [
            Vec2::new(-self.w / 2.0, -self.h / 2.0),  // Top-left
            Vec2::new(self.w / 2.0, -self.h / 2.0),   // Top-right
            Vec2::new(self.w / 2.0, self.h / 2.0),    // Bottom-right
            Vec2::new(-self.w / 2.0, self.h / 2.0),   // Bottom-left
        ]
    }
    
    /// Check if a point is inside this rectangle (centered at origin)
    pub fn contains_point(&self, point: Vec2) -> bool {
        let half_w = self.w / 2.0;
        let half_h = self.h / 2.0;
        
        point.x >= -half_w && point.x <= half_w &&
        point.y >= -half_h && point.y <= half_h
    }
}
```

#### Edge Path Computation Helpers

```rust
// In graphene_layout/src/geometry.rs (new file)

use graphene_core::{math::Vec2, math::Size2};

/// Compute the midpoint of a curve for label positioning
pub fn compute_curve_midpoint(
    source: Vec2,
    target: Vec2,
    style: EdgeCurveStyle,
    curvature: f32,
) -> Vec2 {
    match style {
        EdgeCurveStyle::Straight | EdgeCurveStyle::Taxi => {
            (source + target) * 0.5
        }
        EdgeCurveStyle::Bezier | EdgeCurveStyle::Segmented => {
            // Quadratic bezier: compute control point then evaluate at t=0.5
            let mid = (source + target) * 0.5;
            let dx = target.x - source.x;
            let dy = target.y - source.y;
            let len = (dx * dx + dy * dy).sqrt();
            
            let ctrl = if len > 0.0 {
                Vec2::new(
                    mid.x - (dy / len) * curvature,
                    mid.y + (dx / len) * curvature,
                )
            } else {
                mid
            };
            
            // Quadratic bezier at t=0.5: (1-t)²P0 + 2t(1-t)P1 + t²P2 with t=0.5
            source * 0.25 + ctrl * 0.5 + target * 0.25
        }
        EdgeCurveStyle::UnbundledBezier(cp1, cp2) => {
            // Quartic bezier evaluation at t=0.5
            source * 0.125 + cp1 * 0.375 + cp2 * 0.375 + target * 0.125
        }
    }
}

/// Compute the clipping point where an edge intersects a node's boundary
pub fn compute_edge_clipping(
    center: Vec2,
    size: Size2,
    direction: Vec2,
) -> Vec2 {
    let w = size.w;
    let h = size.h;
    
    let dx = direction.x;
    let dy = direction.y;
    
    // Handle vertical/horizontal cases
    if dx == 0.0 && dy > 0.0 {
        return Vec2::new(center.x, center.y + h / 2.0);
    }
    if dx == 0.0 && dy < 0.0 {
        return Vec2::new(center.x, center.y - h / 2.0);
    }
    
    let dir_slope = dy / dx;
    let node_slope = h / w;
    
    // Right edge (dx > 0)
    if dx > 0.0 && dir_slope >= -node_slope && dir_slope <= node_slope {
        return Vec2::new(center.x + w / 2.0, center.y + (w * dy / (2.0 * dx)));
    }
    
    // Left edge (dx < 0)
    if dx < 0.0 && dir_slope >= -node_slope && dir_slope <= node_slope {
        return Vec2::new(center.x - w / 2.0, center.y - (w * dy / (2.0 * dx)));
    }
    
    // Top edge (dy > 0)
    if dy > 0.0 && (dir_slope <= -node_slope || dir_slope >= node_slope) {
        return Vec2::new(center.x + (h * dx / (2.0 * dy)), center.y + h / 2.0);
    }
    
    // Bottom edge (dy < 0)
    if dy < 0.0 && (dir_slope <= -node_slope || dir_slope >= node_slope) {
        return Vec2::new(center.x - (h * dx / (2.0 * dy)), center.y - h / 2.0);
    }
    
    // Fallback to center
    center
}

/// Compute taxi routing path points for a Manhattan-style edge
pub fn compute_taxi_path(
    source: Vec2,
    target: Vec2,
) -> (Vec2, Vec2) {
    let mid_x = (source.x + target.x) / 2.0;
    
    (
        Vec2::new(mid_x, source.y),  // First waypoint (horizontal)
        Vec2::new(mid_x, target.y),  // Second waypoint (vertical)
    )
}

/// Compute perpendicular offset for bezier curves
pub fn compute_perpendicular_offset(
    source: Vec2,
    target: Vec2,
    magnitude: f32,
) -> Vec2 {
    let dx = target.x - source.x;
    let dy = target.y - source.y;
    let len = (dx * dx + dy * dy).sqrt();
    
    if len > 0.0 {
        // Perpendicular vector rotated 90 degrees counter-clockwise
        Vec2::new(-dy / len, dx / len) * magnitude
    } else {
        Vec2::default()
    }
}
```

---

## Integration with `graphene_gpui`

The rendering module should use these new helpers:

```rust
// In graphene_gpui/src/render/graph_canvas.rs

use graphene_layout::{compute_curve_midpoint, compute_edge_clipping};
use graphene_core::math::{Vec2, Size2};

// Replace inline closure with library function call:
let label_position = compute_curve_midpoint(
    pos_src,
    clipped_tgt,
    curve_style,
    cfg.edge_curvature,
);
```

---

## Migration Strategy

1. **Phase 1** (Hierarchy API) → First priority
   - No breaking changes to existing code
   - Can be used immediately by `interactive_demo`
   
2. **Phase 2** (Live Simulation)
   - Create new module alongside existing layouts
   - Deprecate app_physics logic gradually
   
3. **Phase 3** (Geometry Helpers)
   - Purely additive to `graphene_core::math`
   - Can replace inline implementations

---

## Testing Strategy

For each phase:

1. Unit tests in respective crate (`graphene_core`, `graphene_layout`)
2. Integration tests in `interactive_demo` using new APIs
3. Benchmark suite for performance comparisons

Example test:

```rust
// In graphene_layout/src/livesim.rs

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_live_simulation_tick() {
        let mut state = GraphState::new();
        
        // Add some nodes
        let n1 = state.add_node(NodeData::default(), ComputedStyle::default());
        let n2 = state.add_node(NodeData::default(), ComputedStyle::default());
        
        // Set initial positions
        state.positions.set(0, Vec2::new(0.0, 0.0));
        state.positions.set(1, Vec2::new(100.0, 0.0));
        
        let mut sim = LiveForceSimulation::new(&mut state);
        let initial_dist = (state.positions.get(1) - state.positions.get(0)).len();
        
        // Tick simulation
        sim.tick(&mut state);
        
        let final_dist = (state.positions.get(1) - state.positions.get(0)).len();
        
        // Spring should have pulled nodes closer
        assert!(final_dist < initial_dist);
    }
}
```

---

## Breaking Changes

**None expected.** This is purely additive:

- Existing APIs remain unchanged
- New methods are additions to `GraphState`
- New modules are added without removing existing functionality
- Deprecation notices can be added before removal (future work)
