use crate::basic::{CircleLayout, GridLayout};
use crate::cose::CoseLayout;
use crate::fcose::FCoseLayout;
use crate::force::ForceDirectedLayout;
use crate::hierarchical::SugiyamaLayout;
use crate::livesim::{LiveForceSimulation, RenderSnapshot};
use crate::traits::Layout;
use graphene_analysis::GraphAnalysisReport;
use graphene_core::math::{Size2, Vec2};
use graphene_core::{EdgeData, GraphState, NodeId};
use std::sync::mpsc::{channel, Sender};
use std::sync::{Arc, RwLock};
use std::thread::{spawn, JoinHandle};

/// Supported layout algorithms for the background GraphEngine thread.
pub enum LayoutCommand {
    Cose(CoseLayout),
    FCose(FCoseLayout),
    ForceDirected(ForceDirectedLayout),
    Circle(CircleLayout),
    Grid(GridLayout),
    Sugiyama(SugiyamaLayout),
}

/// Asynchronous commands sent from the main UI thread to the GraphEngine thread.
pub enum GraphCommand<S: Copy + Send + 'static> {
    AddNode { pos: Vec2, size: Size2, data: S },
    AddEdge { source: NodeId, target: NodeId, data: EdgeData },
    RemoveNode(NodeId),
    RemoveEdge(graphene_core::EdgeId),
    SetPosition { id: NodeId, pos: Vec2 },
    TranslateNode { id: NodeId, delta: Vec2 },
    LoadPreset(GraphState<S>),
    RunLayout(LayoutCommand),
    RunAnalysis { is_directed: bool },
    StartLiveSim(LiveForceSimulation),
    StopLiveSim,
    StepLiveSim,
    SetUiMode(bool),
    SetNodeLabel { id: NodeId, label: String },
    UpdateCachedNodeSize { id: NodeId, size: Size2 },
    QueryState(Sender<GraphState<S>>),
    Shutdown,
}

/// Handle held by the main UI thread.
/// Exposes non-blocking snapshot reads and async command dispatching.
pub struct GraphEngineHandle<S: Copy + Send + 'static> {
    snapshot: Arc<RwLock<RenderSnapshot>>,
    analysis_report: Arc<RwLock<Option<GraphAnalysisReport>>>,
    command_tx: Sender<GraphCommand<S>>,
    engine_thread: Option<JoinHandle<()>>,
}

impl<S: Copy + Default + Send + Sync + 'static> GraphEngineHandle<S> {
    /// Spawn the GraphEngine on a dedicated background thread, taking exclusive ownership of `initial_state`.
    pub fn spawn(initial_state: GraphState<S>) -> Self {
        let (command_tx, command_rx) = channel::<GraphCommand<S>>();

        let n = initial_state.node_index_to_id.len();
        let initial_positions: Vec<Vec2> = (0..n).map(|i| *initial_state.positions.get(i)).collect();
        let initial_sizes: Vec<Size2> = (0..n).map(|i| *initial_state.sizes.get(i)).collect();

        let snapshot = Arc::new(RwLock::new(RenderSnapshot {
            positions: initial_positions,
            sizes: initial_sizes,
            version: 0,
            is_ui_mode: initial_state.is_ui_mode,
        }));
        let analysis_report = Arc::new(RwLock::new(None));

        let snapshot_clone = Arc::clone(&snapshot);
        let analysis_report_clone = Arc::clone(&analysis_report);

        let engine_thread = spawn(move || {
            let mut state = initial_state;
            let mut active_sim: Option<LiveForceSimulation> = None;
            let mut version_counter = 0u64;

            loop {
                if active_sim.is_some() {
                    while let Ok(cmd) = command_rx.try_recv() {
                        if !Self::process_command(&mut state, &mut active_sim, cmd, &snapshot_clone, &analysis_report_clone, &mut version_counter) {
                            return;
                        }
                    }
                    if let Some(ref mut sim) = active_sim {
                        sim.tick(&mut state);
                        version_counter += 1;
                        Self::publish_snapshot(&state, &snapshot_clone, version_counter);
                    }
                } else {
                    match command_rx.recv() {
                        Ok(cmd) => {
                            if !Self::process_command(&mut state, &mut active_sim, cmd, &snapshot_clone, &analysis_report_clone, &mut version_counter) {
                                return;
                            }
                        }
                        Err(_) => break,
                    }
                }
            }
        });

        Self {
            snapshot,
            analysis_report,
            command_tx,
            engine_thread: Some(engine_thread),
        }
    }

    fn process_command(
        state: &mut GraphState<S>,
        active_sim: &mut Option<LiveForceSimulation>,
        cmd: GraphCommand<S>,
        snapshot: &Arc<RwLock<RenderSnapshot>>,
        analysis_report: &Arc<RwLock<Option<GraphAnalysisReport>>>,
        version: &mut u64,
    ) -> bool {
        match cmd {
            GraphCommand::AddNode { pos, size, data: _ } => {
                let _id = state.add_node(pos, size);
                *version += 1;
                Self::publish_snapshot(state, snapshot, *version);
            }
            GraphCommand::AddEdge { source, target, data } => {
                state.add_edge(source, target, data);
                *version += 1;
                Self::publish_snapshot(state, snapshot, *version);
            }
            GraphCommand::RemoveNode(node_id) => {
                state.remove_node(node_id);
                *version += 1;
                Self::publish_snapshot(state, snapshot, *version);
            }
            GraphCommand::RemoveEdge(edge_id) => {
                state.remove_edge(edge_id);
                *version += 1;
                Self::publish_snapshot(state, snapshot, *version);
            }
            GraphCommand::SetPosition { id, pos } => {
                if let Some(&idx) = state.node_keys.get(id) {
                    state.positions.set(idx, pos);
                    *version += 1;
                    Self::publish_snapshot(state, snapshot, *version);
                }
            }
            GraphCommand::TranslateNode { id, delta } => {
                if let Some(&idx) = state.node_keys.get(id) {
                    let old_pos = *state.positions.get(idx);
                    state.positions.set(idx, old_pos + delta);
                    *version += 1;
                    Self::publish_snapshot(state, snapshot, *version);
                }
            }
            GraphCommand::LoadPreset(new_state) => {
                *state = new_state;
                *version += 1;
                Self::publish_snapshot(state, snapshot, *version);
            }
            GraphCommand::RunLayout(layout_cmd) => {
                match layout_cmd {
                    LayoutCommand::Cose(mut l) => l.compute(state),
                    LayoutCommand::FCose(mut l) => l.compute(state),
                    LayoutCommand::ForceDirected(mut l) => l.compute(state),
                    LayoutCommand::Circle(mut l) => l.compute(state),
                    LayoutCommand::Grid(mut l) => l.compute(state),
                    LayoutCommand::Sugiyama(mut l) => l.compute(state),
                }
                *version += 1;
                Self::publish_snapshot(state, snapshot, *version);
            }
            GraphCommand::RunAnalysis { is_directed } => {
                // Spawn a sub-thread off the Engine thread to analyze a state snapshot asynchronously
                let mut state_snapshot = state.clone();
                state_snapshot.set_ui_mode(false);
                let report_arc = Arc::clone(analysis_report);
                spawn(move || {
                    let report = GraphAnalysisReport::analyze(&state_snapshot, is_directed);
                    if let Ok(mut lock) = report_arc.write() {
                        *lock = Some(report);
                    }
                });
            }
            GraphCommand::StartLiveSim(sim) => {
                *active_sim = Some(sim);
            }
            GraphCommand::StopLiveSim => {
                *active_sim = None;
            }
            GraphCommand::StepLiveSim => {
                if let Some(ref mut sim) = active_sim {
                    sim.tick(state);
                    *version += 1;
                    Self::publish_snapshot(state, snapshot, *version);
                }
            }
            GraphCommand::SetUiMode(is_ui) => {
                state.set_ui_mode(is_ui);
                *version += 1;
                Self::publish_snapshot(state, snapshot, *version);
            }
            GraphCommand::SetNodeLabel { id, label } => {
                state.set_node_label(id, &label);
                *version += 1;
                Self::publish_snapshot(state, snapshot, *version);
            }
            GraphCommand::UpdateCachedNodeSize { id, size } => {
                state.update_cached_node_size(id, size);
                *version += 1;
                Self::publish_snapshot(state, snapshot, *version);
            }
            GraphCommand::QueryState(tx) => {
                let _ = tx.send(state.clone());
            }
            GraphCommand::Shutdown => return false,
        }
        true
    }

    fn publish_snapshot(
        state: &GraphState<S>,
        snapshot: &Arc<RwLock<RenderSnapshot>>,
        version: u64,
    ) {
        let n = state.node_index_to_id.len();
        let positions: Vec<Vec2> = (0..n).map(|i| *state.positions.get(i)).collect();
        let sizes: Vec<Size2> = (0..n).map(|i| *state.sizes.get(i)).collect();

        if let Ok(mut lock) = snapshot.write() {
            lock.positions = positions;
            lock.sizes = sizes;
            lock.version = version;
            lock.is_ui_mode = state.is_ui_mode;
        }
    }

    /// Read the latest frame buffer snapshot. Instantaneous O(1) read lock.
    pub fn latest_snapshot(&self) -> RenderSnapshot {
        self.snapshot
            .read()
            .map(|guard| guard.clone())
            .unwrap_or_default()
    }

    /// Read the latest analysis report if available. Instantaneous O(1) read lock.
    pub fn latest_analysis_report(&self) -> Option<GraphAnalysisReport> {
        self.analysis_report
            .read()
            .map(|guard| guard.clone())
            .unwrap_or_default()
    }

    /// Send an async command to the GraphEngine thread.
    pub fn send_command(&self, cmd: GraphCommand<S>) -> Result<(), std::sync::mpsc::SendError<GraphCommand<S>>> {
        self.command_tx.send(cmd)
    }

    /// Asynchronously trigger graph analysis on a sub-thread spawned by the Engine thread.
    pub fn run_analysis(&self, is_directed: bool) {
        let _ = self.send_command(GraphCommand::RunAnalysis { is_directed });
    }

    /// Asynchronously drag/set position of a node from the UI thread.
    pub fn drag_node(&self, id: NodeId, pos: Vec2) {
        let _ = self.send_command(GraphCommand::SetPosition { id, pos });
    }

    /// Asynchronously load a new graph preset.
    pub fn load_preset(&self, preset_state: GraphState<S>) {
        let _ = self.send_command(GraphCommand::LoadPreset(preset_state));
    }

    /// Asynchronously run a layout algorithm on the background thread.
    pub fn run_layout(&self, layout: LayoutCommand) {
        let _ = self.send_command(GraphCommand::RunLayout(layout));
    }

    /// Shutdown the background GraphEngine thread.
    pub fn shutdown(mut self) {
        let _ = self.command_tx.send(GraphCommand::Shutdown);
        if let Some(handle) = self.engine_thread.take() {
            let _ = handle.join();
        }
    }
}
