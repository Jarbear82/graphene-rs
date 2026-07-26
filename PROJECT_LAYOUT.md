# Graphene-RS Project Layout Documentation

## Overview

This is a graph visualization library built in Rust with GPUI (Graphical Platform User Interface). The project follows a modular architecture with clear separation of concerns between core data structures, layout algorithms, styling, and UI rendering.

## Directory Structure

```
graphene-rs/
├── Cargo.toml                      # Workspace configuration
├── Cargo.lock                       # Dependency lockfile
├── AGENTS.md                        # Agent documentation
├── crates/                          # Core library crates
│   ├── graphene_algorithms/         # Graph algorithms (centrality, pathfinding, etc.)
│   ├── graphene_analysis/           # Graph analysis utilities
│   ├── graphene_core/               # Core data structures & types
│   ├── graphene_gpui/               # GPUI-specific rendering & interaction
│   ├── graphene_layout/             # Layout algorithms
│   └── graphene_style/              # Styling system
├── examples/                        # Example applications
│   ├── Cargo.toml                   # Examples workspace dependencies
│   └── src/bin/
│       ├── interactive_demo/        # Main interactive visualizer
│       │   ├── main.rs              # Application entry point
│       │   ├── app.rs               # Main application logic
│       │   ├── app_physics.rs       # Physics engine (collision detection, forces)
│       │   ├── render.rs            # UI rendering (canvas, panels)
│       │   ├── render_analysis.rs   # Analysis visualization
│       │   ├── render_left.rs       # Left panel rendering
│       │   ├── render_right.rs      # Right panel rendering
│       │   └── theme.rs             # Theme management & helper functions
│       └── headless_algo_viz.rs     # Headless algorithm visualization
└── target/                          # Build artifacts
```

## Core Crates

### 1. graphene_core

**Purpose**: Core data structures and types for graph representation

**Key Files**:
- `src/lib.rs` - Module exports
- `src/graphs.rs` - Graph definitions and operations
- `src/state/mod.rs` - `GraphState` struct (main state container)
- `src/types.rs` - Type definitions (`NodeId`, `EdgeId`, `Vec2`, etc.)
- `src/math.rs` - Math utilities (`Size2`, `Vec2`)
- `src/view.rs` - Graph view abstractions

**Key Structures**:
```rust
GraphState<S: Copy> {
    topology: GraphTopology,           // Node/edge connections
    visuals: GraphVisuals<S>,          // Visual properties
    animation: GraphAnimation,         // Animation state
    
    node_keys: SlotMap<NodeId, usize>,  // ID to index mapping
    node_index_to_id: Vec<NodeId>,      // Index to ID mapping
    positions: DenseStorage<Vec2>,      // Node positions
    sizes: DenseStorage<Size2>,         // Node sizes
    edges: DenseStorage<EdgeData>,      // Edge data
    
    hierarchy: Hierarchy,              // Parent-child relationships
    computed_styles: DenseStorage<S>,  // Computed styles
}
```

### 2. graphene_layout

**Purpose**: Layout algorithms for positioning nodes

**Key Files**:
- `src/lib.rs` - Module exports (re-exports all layout types)
- `src/traits.rs` - `Layout` trait and utility functions
- Various layout implementations:
  - `force.rs` - Force-directed layouts
  - `cose.rs` - CoSE (Compound Spring Embedder)
  - `fcose.rs` - Fast CoSE
  - `hierarchical.rs` - Sugiyama layout for DAGs
  - `bipartite.rs` - Bipartite layout
  - `tree.rs` - Reingold-Tilford tree layout
  - `compound.rs` - Compound node layouts
  - `multigraph.rs` - Multi-edge routing

**Dependencies**: Only depends on `graphene_core`

**Layout Trait**:
```rust
pub trait Layout<S: Copy = ()> {
    fn compute(&mut self, state: &mut GraphState<S>);
}
```

### 3. graphene_style

**Purpose**: Styling system for nodes and edges

**Key Files**:
- `src/lib.rs` - Complete styling system

**Key Structures**:
```rust
// Style definitions
NodeStyle { fill_color, border_color, border_width, shape, label, ... }
EdgeStyle { line_color, line_width, curve_style, label, ... }

// Styling engine
StylingEngine {
    rule_engine: RuleEngine,      // CSS-like selector matching
    node_bypasses, edge_bypasses // Per-node/edge style overrides
}

// Theme system
Theme { bg, panel_bg, border, accent, text, ... }
```

**Features**:
- CSS-like selector system for styling
- Support for classes and state-based styling
- Built-in themes (Catppuccin Mocha, Gruvbox Dark, One Dark, GitHub Light)
- Data-driven styling (map data values to colors/sizes)

### 4. graphene_gpui

**Purpose**: GPUI-specific rendering and interaction

**Key Files**:
- `src/lib.rs` - Module exports
- `src/render/graph_canvas.rs` - Main canvas renderer (`GraphCanvas`)
- `src/convert.rs` - Type conversions
- `src/style_bridge.rs` - Bridge between graphene_style and GPUI types
- `src/interaction/state.rs` - Interaction state management (drag, pan, hit testing)

**Dependencies**: Depends on `graphene_core`, `graphene_style`, `graphene_layout`

**Key Structures**:
```rust
GraphCanvas<'a> {
    state: &'a GraphState<S>,
    viewport: Viewport,
    interaction_state: InteractionState,
    theme: Theme,
    selected_node: Option<NodeId>,
    config: CanvasConfig,  // Rendering configuration
}

InteractionState {
    drag_start: Option<(NodeId, Point<f32>, Vec2)>,  // Node being dragged
    pan_origin: Option<Point<f32>>,                    // Pan start position
    spatial_grid: SpatialHashGrid,                     // Spatial hashing for hit testing
    is_box_selecting: bool,
    box_select_rect: Option<Bounds<f32>>,
}

SpatialHashGrid {
    cell_size: f32,
    cells: HashMap<(i32, i32), Vec<NodeId>>,          // Grid-based spatial indexing
}
```

### 5. graphene_algorithms

**Purpose**: Graph algorithms (centrality measures, pathfinding, clustering)

**Key Files**:
- `src/centrality/` - Betweenness, closeness, degree centrality, PageRank
- `src/pathfinding/` - Dijkstra, A*, Bellman-Ford, Floyd-Warshall
- `src/clustering/` - Affinity propagation, hierarchical clustering, K-means
- `src/search_traversal/` - BFS, DFS, Kruskal's MST

**Dependencies**: Only depends on `graphene_core`

### 6. graphene_analysis

**Purpose**: High-level analysis utilities

**Key Files**:
- `src/lib.rs` - Analysis exports
- `src/centrality.rs`, `src/connectivity.rs`, `src/metrics.rs`

**Dependencies**: Depends on `graphene_algorithms`

## Interactive Demo Application

### Architecture

The interactive demo demonstrates how all components work together:

```
Interactive Demo Structure:
┌─────────────────────────────────────────────────────┐
│                   main.rs                           │
│  (Application entry point, window setup)            │
└────────────────┬────────────────────────────────────┘
                 │
                 ▼
┌─────────────────────────────────────────────────────┐
│                    app.rs                           │
│  (DemoApp struct - main state management)           │
│  - Loads fixtures                                   │
│  - Manages layouts                                  │
│  - Handles user input                               │
└────────┬──────────────────┬─────────────────────────┘
         │                  │
         ▼                  ▼
┌────────────────┐  ┌────────────────────┐
│  app_physics.rs│  │    render.rs       │
│  (Physics step)│  │  (UI rendering)    │
└────────────────┘  └────────────────────┘
```

### Physics Engine (`app_physics.rs`)

**Purpose**: Spring-embedded physics simulation for node positioning

**Key Functions**:
```rust
impl DemoApp {
    pub fn run_physics_step(&mut self)     // Single physics step
    pub fn resolve_collisions(&mut self)   // Collision resolution
}
```

**Features**:
- Node repulsion (Coulomb's law)
- Edge attraction (spring forces)
- Gravity towards center
- Hierarchical constraints (parent nodes affect children)
- Collision detection between nodes
- Dragging support

**Dependencies**: Uses `graphene_core::Vec2` and `graphene_layout::find_clipping_point`

### Rendering (`render.rs`)

**Purpose**: GPUI rendering of the application UI

**Key Components**:
```rust
impl Render for DemoApp {
    fn render(&mut self, window: &Window, cx: &Context) -> impl IntoElement {
        // Renders: title bar, canvas view, bottom panel
    }
}
```

**Canvas Rendering**: Uses `graphene_gpui::GraphCanvas` to render nodes and edges

### Theme Management (`theme.rs`)

**Purpose**: Color conversion and theme management

**Key Functions**:
```rust
pub fn color_value_to_gpui_color(color_val: ColorValue) -> gpui::Rgba
pub struct Theme { bg, panel_bg, border, accent, text, text_dim }
impl Theme {
    pub fn from_style(theme: &graphene_style::Theme) -> Self
}
```

## Component Interaction Flow

### 1. Loading a Graph Fixture

```
User selects fixture
         │
         ▼
app.rs: load_preset()
         │
         ▼
Loads fixture data (nodes, edges)
         │
         ▼
Creates GraphState with node positions/sizes
```

### 2. Running a Layout

```
User triggers layout
         │
         ▼
app.rs: run_layout_internal()
         │
         ▼
Selects layout algorithm from graphene_layout
(e.g., ForceDirectedLayout, CircleLayout, etc.)
         │
         ▼
Calls layout.compute(&mut state)
         │
         ▼
Layout updates positions in GraphState
```

### 3. Physics Simulation

```
Physics enabled?
         │
         ┌───yes────┐
         │        │
         ▼        ▼
    run_physics_step()  (every frame)
         │
         ▼
Computes forces (repulsion, attraction, gravity)
         │
         ▼
Updates positions with force application
         │
         ▼
resolve_collisions() to prevent overlap
```

### 4. Rendering Pipeline

```
UI render trigger
         │
         ▼
render.rs: render_canvas_view()
         │
         ▼
Creates GraphCanvas with:
  - state from DemoApp
  - current theme
  - config (edges, arrows, styles)
         │
         ▼
graphene_gpui::GraphCanvas renders nodes/edges
         │
         ▼
GPUI draws to window
```

## Dependencies Map

```
graphene_algorithms ─┐
                     ├──> examples/interactive_demo
graphene_analysis ───┘

graphene_core ────────────────────────> (all other crates)

graphene_layout ──> graphene_gpui ──> examples/interactive_demo
                    ^              │
                    └──> examples/headless_algo_viz

graphene_style ───> graphene_gpui ──> examples/interactive_demo
                    ^              
                    └──> examples/headless_algo_viz
```

## Key Design Patterns

1. **State Management**: Single `GraphState` struct holds all graph data
2. **Trait-based Layouts**: All layouts implement the `Layout` trait
3. **Generic Styling**: `GraphState<S>` allows custom style types
4. **Separation of Concerns**: Core, layout, style, and render are separate crates
5. **Data-driven Styling**: CSS-like selectors for styling based on data

## Usage Example (from examples)

```rust
use graphene_core::{GraphState, Vec2};
use graphene_layout::ForceDirectedLayout;

// Create graph state
let mut state = GraphState::new();
state.add_node(Vec2::default(), Size2::new(50.0, 50.0));
state.add_edge(node1, node2, EdgeData::default());

// Run layout
let mut layout = ForceDirectedLayout {
    iterations: 150,
    ideal_length: 50.0,
    ..Default::default()
};
layout.compute(&mut state);

// Render with graphene_gpui
let canvas = GraphCanvas::new(&state, theme, config);
```
