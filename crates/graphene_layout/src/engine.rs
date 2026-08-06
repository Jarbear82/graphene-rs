use crate::basic::{CircleLayout, GridLayout};
use crate::cose::CoseLayout;
use crate::fcose::FCoseLayout;
use crate::force::ForceDirectedLayout;
use crate::hierarchical::SugiyamaLayout;
use crate::livesim::{LiveForceSimulation, RenderSnapshot};
use crate::traits::Layout;
use crate::ConcentricHubLayout;
use crate::KamadaKawaiLayout;
use crate::MdsLayout;
use crate::ReingoldTilfordLayout;
use graphene_analysis::GraphAnalysisReport;
use graphene_core::math::{Size2, Vec2};
use graphene_core::{EdgeData, GraphState, NodeId, UndoRedoManager};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{channel, Sender};
use std::sync::{Arc, RwLock};
use std::thread::{spawn, JoinHandle};

use crate::fruchterman_reingold::FruchtermanReingoldLayout;
use crate::multilevel::MultilevelLayout;
use crate::tutte::TutteBarycentricLayout;

/// Supported layout algorithms for the background GraphEngine thread.
pub enum LayoutCommand {
    Cose(CoseLayout),
    FCose(FCoseLayout),
    ForceDirected(ForceDirectedLayout),
    Circle(CircleLayout),
    Grid(GridLayout),
    Sugiyama(SugiyamaLayout),
    KamadaKawai(KamadaKawaiLayout),
    ReingoldTilford(ReingoldTilfordLayout),
    Mds(MdsLayout),
    Concentric(ConcentricHubLayout),
    FruchtermanReingold(FruchtermanReingoldLayout),
    Tutte(TutteBarycentricLayout),
    MultilevelForce(MultilevelLayout<ForceDirectedLayout>),
    MaximalShift(crate::planar_shift::MaximalShiftLayout),
    CircularAdvanced(crate::circular_advanced::CircularAdvancedLayout),
}

impl LayoutCommand {
    pub const ALL_NAMES: &'static [&'static str] = &[
        "Circle",
        "ForceDirected",
        "CoSE",
        "KamadaKawai",
        "Sugiyama",
        "ReingoldTilford",
        "MDS",
        "Grid",
        "Concentric",
        "fCoSE",
        "FruchtermanReingold",
        "Tutte",
        "MultilevelForce",
        "MaximalShift",
        "CircularAdvanced",
    ];

    pub fn from_name(name: &str, iterations: usize) -> Option<Self> {
        match name {
            "Circle" => Some(Self::Circle(CircleLayout::default())),
            "ForceDirected" => Some(Self::ForceDirected(
                ForceDirectedLayout::default().with_iterations(iterations),
            )),
            "CoSE" => Some(Self::Cose(CoseLayout::default())),
            "KamadaKawai" => Some(Self::KamadaKawai(
                KamadaKawaiLayout::default().with_iterations(iterations),
            )),
            "Sugiyama" => Some(Self::Sugiyama(SugiyamaLayout::default())),
            "ReingoldTilford" => Some(Self::ReingoldTilford(ReingoldTilfordLayout::default())),
            "MDS" => Some(Self::Mds(MdsLayout::default().with_iterations(iterations))),
            "Grid" => Some(Self::Grid(GridLayout::default())),
            "Concentric" => Some(Self::Concentric(ConcentricHubLayout::default())),
            "fCoSE" => Some(Self::FCose(
                FCoseLayout::default().with_iterations(iterations),
            )),
            "FruchtermanReingold" => Some(Self::FruchtermanReingold(
                FruchtermanReingoldLayout::default().with_iterations(iterations),
            )),
            "Tutte" => Some(Self::Tutte(
                TutteBarycentricLayout::default().with_max_iterations(iterations),
            )),
            "MultilevelForce" => Some(Self::MultilevelForce(MultilevelLayout::new(
                ForceDirectedLayout::default().with_iterations(20),
            ))),
            "MaximalShift" => Some(Self::MaximalShift(crate::planar_shift::MaximalShiftLayout::default())),
            "CircularAdvanced" => Some(Self::CircularAdvanced(crate::circular_advanced::CircularAdvancedLayout::default())),
            _ => None,
        }
    }
}

/// Phase of an interactive node drag gesture.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DragPhase {
    Begin,
    Update,
    End,
}

/// Asynchronous commands sent from the main UI thread to the GraphEngine thread.
pub enum GraphCommand<S: Copy + Send + 'static> {
    AddNode {
        pos: Vec2,
        size: Size2,
        data: S,
        label: Option<String>,
    },
    AddEdge {
        source: NodeId,
        target: NodeId,
        data: EdgeData,
    },
    RemoveNode(NodeId),
    RemoveEdge(graphene_core::EdgeId),
    SetPosition {
        id: NodeId,
        pos: Vec2,
    },
    DragNodeTarget {
        id: NodeId,
        target_pos: Vec2,
        phase: DragPhase,
    },
    TranslateNode {
        id: NodeId,
        delta: Vec2,
    },
    TranslateSubtree {
        id: NodeId,
        delta: Vec2,
    },
    Reparent {
        child: NodeId,
        parent: Option<NodeId>,
    },
    SetNodeLabel {
        id: NodeId,
        label: String,
    },
    SetNodeData {
        id: NodeId,
        data: S,
    },
    SetNodeExpansionMode {
        id: NodeId,
        mode: graphene_core::DataExpansionMode,
    },
    Undo,
    Redo,
    LoadPreset(GraphState<S>),
    RunLayout(LayoutCommand),
    RunAnalysis {
        is_directed: bool,
    },
    StartLiveSim(LiveForceSimulation),
    StopLiveSim,
    StepLiveSim,
    StepLiveSimN(usize),
    StepLayoutPhase(LayoutCommand),
    UpdateLiveSimParam(crate::livesim::LiveSimParam),
    SetUiMode(bool),
    UpdateCachedNodeSize {
        id: NodeId,
        size: Size2,
    },
    QueryState(Sender<GraphState<S>>),
    Shutdown,
}

/// Reactive updates sent from the GraphEngine thread to the UI thread.
#[derive(Debug, Clone)]
pub enum GraphUpdate<S: Copy + Send + 'static> {
    NodeAdded {
        id: NodeId,
        pos: Vec2,
        size: Size2,
        data: S,
        label: String,
        parent: Option<NodeId>,
    },
    NodeRemoved {
        id: NodeId,
    },
    EdgeAdded {
        id: graphene_core::EdgeId,
        source: NodeId,
        target: NodeId,
        data: EdgeData,
    },
    EdgeRemoved {
        id: graphene_core::EdgeId,
    },
    NodeUpdated {
        id: NodeId,
        pos: Option<Vec2>,
        size: Option<Size2>,
        label: Option<String>,
        data: Option<S>,
    },
    NodeReparented {
        child: NodeId,
        parent: Option<NodeId>,
    },
    Snapshot(RenderSnapshot),
    AnalysisReady(GraphAnalysisReport),
    CommandRejected {
        reason: String,
    },
    StateVersion(u64),
    PresetLoaded(GraphState<S>),
    UndoRedoStateChanged {
        can_undo: bool,
        can_redo: bool,
    },
}

/// Current execution activity of the background GraphEngine worker thread.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EngineWorkerState {
    Idle,
    RunningPhysics,
    ComputingLayout,
    AnalyzingGraph,
}

impl EngineWorkerState {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Idle => "Idle (Thread Waiting)",
            Self::RunningPhysics => "Running Live Physics",
            Self::ComputingLayout => "Computing Static Layout",
            Self::AnalyzingGraph => "Analyzing Graph Topology",
        }
    }
}

/// Handle held by the main UI thread.
/// Exposes non-blocking snapshot reads and async command dispatching.
pub struct GraphEngineHandle<S: Copy + Send + 'static> {
    snapshot: Arc<RwLock<RenderSnapshot>>,
    analysis_report: Arc<RwLock<Option<GraphAnalysisReport>>>,
    command_tx: Sender<GraphCommand<S>>,
    update_rx: std::sync::mpsc::Receiver<GraphUpdate<S>>,
    engine_thread: Option<JoinHandle<()>>,
    active_worker_threads: Arc<AtomicUsize>,
    worker_activity: Arc<AtomicUsize>,
}

impl<S: Copy + Default + Send + Sync + 'static> GraphEngineHandle<S> {
    /// Spawn the GraphEngine on a dedicated background thread, taking exclusive ownership of `initial_state`.
    pub fn spawn(initial_state: GraphState<S>) -> Self {
        let (command_tx, command_rx) = channel::<GraphCommand<S>>();
        let (update_tx, update_rx) = channel::<GraphUpdate<S>>();

        let n = initial_state.node_index_to_id.len();
        let initial_positions: Vec<Vec2> =
            (0..n).map(|i| *initial_state.positions.get(i)).collect();
        let initial_sizes: Vec<Size2> = (0..n).map(|i| *initial_state.sizes.get(i)).collect();

        let snapshot = Arc::new(RwLock::new(RenderSnapshot {
            positions: initial_positions,
            sizes: initial_sizes,
            version: 0,
            is_ui_mode: initial_state.is_ui_mode,
        }));
        let analysis_report = Arc::new(RwLock::new(None));
        let active_worker_threads = Arc::new(AtomicUsize::new(0));
        let worker_activity = Arc::new(AtomicUsize::new(0));

        let snapshot_clone = Arc::clone(&snapshot);
        let analysis_report_clone = Arc::clone(&analysis_report);
        let threads_counter_clone = Arc::clone(&active_worker_threads);
        let activity_counter_clone = Arc::clone(&worker_activity);

        let engine_thread = spawn(move || {
            let mut state = initial_state;
            let mut undo_redo = UndoRedoManager::<S>::new();
            let mut active_sim: Option<LiveForceSimulation> = None;
            let mut version_counter = 0u64;

            let _ = update_tx.send(GraphUpdate::PresetLoaded(state.clone()));

            loop {
                if active_sim.is_some() {
                    activity_counter_clone.store(1, Ordering::Relaxed);
                    let mut coalesced_drags = std::collections::HashMap::<NodeId, Vec2>::new();
                    while let Ok(cmd) = command_rx.try_recv() {
                        if let GraphCommand::DragNodeTarget { id, target_pos, phase: DragPhase::Update } = cmd {
                            coalesced_drags.insert(id, target_pos);
                        } else {
                            if !Self::process_command(
                                &mut state,
                                &mut undo_redo,
                                &mut active_sim,
                                cmd,
                                &snapshot_clone,
                                &analysis_report_clone,
                                &update_tx,
                                &threads_counter_clone,
                                &activity_counter_clone,
                                &mut version_counter,
                            ) {
                                return;
                            }
                        }
                    }
                    for (id, target_pos) in coalesced_drags {
                        Self::process_command(
                            &mut state,
                            &mut undo_redo,
                            &mut active_sim,
                            GraphCommand::DragNodeTarget {
                                id,
                                target_pos,
                                phase: DragPhase::Update,
                            },
                            &snapshot_clone,
                            &analysis_report_clone,
                            &update_tx,
                            &threads_counter_clone,
                            &activity_counter_clone,
                            &mut version_counter,
                        );
                    }
                    if let Some(ref mut sim) = active_sim {
                        sim.tick(&mut state);
                        version_counter += 1;
                        Self::publish_snapshot(&state, &snapshot_clone, version_counter);
                    }
                } else {
                    activity_counter_clone.store(0, Ordering::Relaxed);
                    match command_rx.recv() {
                        Ok(cmd) => {
                            if !Self::process_command(
                                &mut state,
                                &mut undo_redo,
                                &mut active_sim,
                                cmd,
                                &snapshot_clone,
                                &analysis_report_clone,
                                &update_tx,
                                &threads_counter_clone,
                                &activity_counter_clone,
                                &mut version_counter,
                            ) {
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
            update_rx,
            engine_thread: Some(engine_thread),
            active_worker_threads,
            worker_activity,
        }
    }

    fn process_command(
        state: &mut GraphState<S>,
        undo_redo: &mut UndoRedoManager<S>,
        active_sim: &mut Option<LiveForceSimulation>,
        cmd: GraphCommand<S>,
        snapshot: &Arc<RwLock<RenderSnapshot>>,
        analysis_report: &Arc<RwLock<Option<GraphAnalysisReport>>>,
        update_tx: &Sender<GraphUpdate<S>>,
        active_threads: &Arc<AtomicUsize>,
        worker_activity: &Arc<AtomicUsize>,
        version: &mut u64,
    ) -> bool {
        match cmd {
            GraphCommand::AddNode { pos, size, data, label } => {
                undo_redo.record_state(state);
                let id = state.add_node(pos, size);
                if let Some(ref lbl) = label {
                    state.set_node_label(id, lbl);
                }
                if let Some(&idx) = state.node_keys.get(id) {
                    state.computed_styles.set(idx, data);
                }
                *version += 1;
                Self::publish_snapshot(state, snapshot, *version);
                let lbl_val = state
                    .get_node_label(id)
                    .map(|s| s.to_string())
                    .unwrap_or_default();
                let parent = state
                    .node_keys
                    .get(id)
                    .and_then(|&idx| *state.hierarchy.parent.get(idx));
                let _ = update_tx.send(GraphUpdate::NodeAdded {
                    id,
                    pos,
                    size,
                    data,
                    label: lbl_val,
                    parent,
                });
            }
            GraphCommand::AddEdge {
                source,
                target,
                data,
            } => {
                undo_redo.record_state(state);
                let edge_id = state.add_edge(source, target, data.clone());
                *version += 1;
                Self::publish_snapshot(state, snapshot, *version);
                let _ = update_tx.send(GraphUpdate::EdgeAdded {
                    id: edge_id,
                    source,
                    target,
                    data,
                });
            }
            GraphCommand::RemoveNode(node_id) => {
                undo_redo.record_state(state);
                state.remove_node(node_id);
                *version += 1;
                Self::publish_snapshot(state, snapshot, *version);
                let _ = update_tx.send(GraphUpdate::NodeRemoved { id: node_id });
            }
            GraphCommand::RemoveEdge(edge_id) => {
                undo_redo.record_state(state);
                state.remove_edge(edge_id);
                *version += 1;
                Self::publish_snapshot(state, snapshot, *version);
                let _ = update_tx.send(GraphUpdate::EdgeRemoved { id: edge_id });
            }
            GraphCommand::SetPosition { id, pos } => {
                if let Some(&idx) = state.node_keys.get(id) {
                    undo_redo.record_state(state);
                    state.positions.set(idx, pos);
                    *version += 1;
                    Self::publish_snapshot(state, snapshot, *version);
                    let _ = update_tx.send(GraphUpdate::NodeUpdated {
                        id,
                        pos: Some(pos),
                        size: None,
                        label: None,
                        data: None,
                    });
                }
            }
            GraphCommand::DragNodeTarget { id, target_pos, phase } => {
                if let Some(&idx) = state.node_keys.get(id) {
                    match phase {
                        DragPhase::Begin => {
                            if let Some(ref mut sim) = active_sim {
                                sim.pin_node(id);
                            }
                            state.positions.set(idx, target_pos);
                            *version += 1;
                            Self::publish_snapshot(state, snapshot, *version);
                        }
                        DragPhase::Update => {
                            state.positions.set(idx, target_pos);
                            *version += 1;
                            Self::publish_snapshot(state, snapshot, *version);
                        }
                        DragPhase::End => {
                            if let Some(ref mut sim) = active_sim {
                                sim.unpin_node(id);
                            }
                            undo_redo.record_state(state);
                            state.positions.set(idx, target_pos);
                            *version += 1;
                            Self::publish_snapshot(state, snapshot, *version);
                            let _ = update_tx.send(GraphUpdate::NodeUpdated {
                                id,
                                pos: Some(target_pos),
                                size: None,
                                label: None,
                                data: None,
                            });
                        }
                    }
                }
            }
            GraphCommand::TranslateNode { id, delta } => {
                if let Some(&idx) = state.node_keys.get(id) {
                    undo_redo.record_state(state);
                    let old_pos = *state.positions.get(idx);
                    let new_pos = old_pos + delta;
                    state.positions.set(idx, new_pos);
                    *version += 1;
                    Self::publish_snapshot(state, snapshot, *version);
                    let _ = update_tx.send(GraphUpdate::NodeUpdated {
                        id,
                        pos: Some(new_pos),
                        size: None,
                        label: None,
                        data: None,
                    });
                }
            }
            GraphCommand::TranslateSubtree { id, delta } => {
                undo_redo.record_state(state);
                state.translate_node_and_descendants(id, delta);
                *version += 1;
                Self::publish_snapshot(state, snapshot, *version);
                let _ = update_tx.send(GraphUpdate::PresetLoaded(state.clone()));
            }
            GraphCommand::Reparent { child, parent } => {
                undo_redo.record_state(state);
                state.reparent_node(child, parent);
                *version += 1;
                Self::publish_snapshot(state, snapshot, *version);
                let _ = update_tx.send(GraphUpdate::NodeReparented { child, parent });
            }
            GraphCommand::SetNodeLabel { id, label } => {
                undo_redo.record_state(state);
                state.set_node_label(id, &label);
                *version += 1;
                Self::publish_snapshot(state, snapshot, *version);
                let _ = update_tx.send(GraphUpdate::NodeUpdated {
                    id,
                    pos: None,
                    size: None,
                    label: Some(label),
                    data: None,
                });
            }
            GraphCommand::SetNodeData { id, data } => {
                if let Some(&idx) = state.node_keys.get(id) {
                    undo_redo.record_state(state);
                    state.computed_styles.set(idx, data);
                    *version += 1;
                    Self::publish_snapshot(state, snapshot, *version);
                    let _ = update_tx.send(GraphUpdate::NodeUpdated {
                        id,
                        pos: None,
                        size: None,
                        label: None,
                        data: Some(data),
                    });
                }
            }
            GraphCommand::SetNodeExpansionMode { id, mode } => {
                if let Some(&idx) = state.node_keys.get(id) {
                    undo_redo.record_state(state);
                    state.set_node_expansion_mode(id, mode);
                    let new_size = *state.sizes.get(idx);
                    *version += 1;
                    Self::publish_snapshot(state, snapshot, *version);
                    let _ = update_tx.send(GraphUpdate::NodeUpdated {
                        id,
                        pos: None,
                        size: Some(new_size),
                        label: None,
                        data: None,
                    });
                }
            }
            GraphCommand::Undo => {
                if undo_redo.undo(state) {
                    *version += 1;
                    Self::publish_snapshot(state, snapshot, *version);
                    let _ = update_tx.send(GraphUpdate::PresetLoaded(state.clone()));
                }
            }
            GraphCommand::Redo => {
                if undo_redo.redo(state) {
                    *version += 1;
                    Self::publish_snapshot(state, snapshot, *version);
                    let _ = update_tx.send(GraphUpdate::PresetLoaded(state.clone()));
                }
            }
            GraphCommand::LoadPreset(new_state) => {
                undo_redo.record_state(state);
                *state = new_state;
                *version += 1;
                Self::publish_snapshot(state, snapshot, *version);
                let _ = update_tx.send(GraphUpdate::PresetLoaded(state.clone()));
            }
            GraphCommand::RunLayout(layout_cmd) => {
                undo_redo.record_state(state);
                active_threads.fetch_add(1, Ordering::Relaxed);
                worker_activity.store(2, Ordering::Relaxed);
                match layout_cmd {
                    LayoutCommand::Cose(mut l) => l.compute(state),
                    LayoutCommand::FCose(mut l) => l.compute(state),
                    LayoutCommand::ForceDirected(mut l) => l.compute(state),
                    LayoutCommand::Circle(mut l) => l.compute(state),
                    LayoutCommand::Grid(mut l) => l.compute(state),
                    LayoutCommand::Sugiyama(mut l) => l.compute(state),
                    LayoutCommand::Concentric(mut l) => l.compute(state),
                    LayoutCommand::KamadaKawai(mut l) => l.compute(state),
                    LayoutCommand::Mds(mut l) => l.compute(state),
                    LayoutCommand::ReingoldTilford(mut l) => l.compute(state),
                    LayoutCommand::FruchtermanReingold(mut l) => l.compute(state),
                    LayoutCommand::Tutte(mut l) => l.compute(state),
                    LayoutCommand::MultilevelForce(mut l) => l.compute(state),
                    LayoutCommand::MaximalShift(l) => l.apply(state),
                    LayoutCommand::CircularAdvanced(l) => l.apply(state),
                }
                let collapsed = std::collections::HashSet::new();
                crate::collision::finish_layout_epilogue(state, &collapsed, 10.0, 20.0);
                *version += 1;
                Self::publish_snapshot(state, snapshot, *version);
                let _ = update_tx.send(GraphUpdate::PresetLoaded(state.clone()));
                worker_activity.store(if active_sim.is_some() { 1 } else { 0 }, Ordering::Relaxed);
                active_threads.fetch_sub(1, Ordering::Relaxed);
            }
            GraphCommand::RunAnalysis { is_directed } => {
                let mut state_snapshot = state.clone();
                state_snapshot.set_ui_mode(false);
                let report_arc = Arc::clone(analysis_report);
                let update_tx_clone = update_tx.clone();
                let counter = Arc::clone(active_threads);
                counter.fetch_add(1, Ordering::Relaxed);
                spawn(move || {
                    let report = GraphAnalysisReport::analyze(&state_snapshot, is_directed);
                    if let Ok(mut lock) = report_arc.write() {
                        *lock = Some(report.clone());
                    }
                    let _ = update_tx_clone.send(GraphUpdate::AnalysisReady(report));
                    counter.fetch_sub(1, Ordering::Relaxed);
                });
            }
            GraphCommand::StartLiveSim(sim) => {
                if active_sim.is_none() {
                    active_threads.fetch_add(1, Ordering::Relaxed);
                }
                *active_sim = Some(sim);
            }
            GraphCommand::StopLiveSim => {
                if active_sim.is_some() {
                    active_threads.fetch_sub(1, Ordering::Relaxed);
                }
                *active_sim = None;
            }
            GraphCommand::StepLiveSim => {
                if let Some(ref mut sim) = active_sim {
                    sim.tick(state);
                    *version += 1;
                    Self::publish_snapshot(state, snapshot, *version);
                }
            }
            GraphCommand::StepLiveSimN(steps) => {
                if let Some(ref mut sim) = active_sim {
                    for _ in 0..steps {
                        sim.tick(state);
                    }
                    *version += 1;
                    Self::publish_snapshot(state, snapshot, *version);
                }
            }
            GraphCommand::StepLayoutPhase(layout_cmd) => {
                use crate::traits::PhaseSteppableLayout;
                match layout_cmd {
                    LayoutCommand::Sugiyama(mut s) => {
                        s.step_next_phase(state);
                    }
                    LayoutCommand::Cose(mut c) => {
                        c.step_next_phase(state);
                    }
                    LayoutCommand::FCose(mut f) => {
                        f.step_next_phase(state);
                    }
                    other => match other {
                        LayoutCommand::ForceDirected(mut l) => l.compute(state),
                        LayoutCommand::Circle(mut l) => l.compute(state),
                        LayoutCommand::Grid(mut l) => l.compute(state),
                        _ => unreachable!(),
                    },
                }
                *version += 1;
                Self::publish_snapshot(state, snapshot, *version);
                let _ = update_tx.send(GraphUpdate::PresetLoaded(state.clone()));
            }
            GraphCommand::UpdateLiveSimParam(param) => {
                if let Some(ref mut sim) = active_sim {
                    sim.update_param(param);
                }
            }
            GraphCommand::SetUiMode(is_ui) => {
                state.set_ui_mode(is_ui);
                *version += 1;
                Self::publish_snapshot(state, snapshot, *version);
            }
            GraphCommand::UpdateCachedNodeSize { id, size } => {
                state.update_cached_node_size(id, size);
                *version += 1;
                Self::publish_snapshot(state, snapshot, *version);
                let _ = update_tx.send(GraphUpdate::NodeUpdated {
                    id,
                    pos: None,
                    size: Some(size),
                    label: None,
                    data: None,
                });
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

        if let Ok(mut lock) = snapshot.write() {
            lock.positions.clear();
            lock.positions.reserve(n);
            for i in 0..n {
                lock.positions.push(*state.positions.get(i));
            }

            lock.sizes.clear();
            lock.sizes.reserve(n);
            for i in 0..n {
                lock.sizes.push(*state.sizes.get(i));
            }

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

    /// Try receiving a single pending `GraphUpdate` from the engine update channel.
    pub fn try_recv_update(&self) -> Option<GraphUpdate<S>> {
        self.update_rx.try_recv().ok()
    }

    /// Drain all pending `GraphUpdate` messages from the engine update channel.
    pub fn drain_updates(&self) -> Vec<GraphUpdate<S>> {
        let mut updates = Vec::new();
        while let Ok(upd) = self.update_rx.try_recv() {
            updates.push(upd);
        }
        updates
    }

    /// Return the number of currently active background worker threads.
    pub fn active_worker_threads(&self) -> usize {
        self.active_worker_threads.load(Ordering::Relaxed)
    }

    /// Return the current activity state of the background worker thread.
    pub fn worker_state(&self) -> EngineWorkerState {
        match self.worker_activity.load(Ordering::Relaxed) {
            1 => EngineWorkerState::RunningPhysics,
            2 => EngineWorkerState::ComputingLayout,
            3 => EngineWorkerState::AnalyzingGraph,
            _ => EngineWorkerState::Idle,
        }
    }

    /// Read the latest analysis report if available. Instantaneous O(1) read lock.
    pub fn latest_analysis_report(&self) -> Option<GraphAnalysisReport> {
        self.analysis_report
            .read()
            .map(|guard| guard.clone())
            .unwrap_or_default()
    }

    /// Send an async command to the GraphEngine thread.
    pub fn send_command(
        &self,
        cmd: GraphCommand<S>,
    ) -> Result<(), std::sync::mpsc::SendError<GraphCommand<S>>> {
        self.command_tx.send(cmd)
    }

    /// Asynchronously trigger graph analysis on a sub-thread.
    pub fn run_analysis(&self, is_directed: bool) {
        let _ = self.send_command(GraphCommand::RunAnalysis { is_directed });
    }

    /// Asynchronously set position of a node.
    pub fn drag_node(&self, id: NodeId, pos: Vec2) {
        let _ = self.send_command(GraphCommand::SetPosition { id, pos });
    }

    /// Asynchronously dispatch a node drag gesture phase target to the background GraphEngine thread.
    pub fn drag_node_target(&self, id: NodeId, target_pos: Vec2, phase: DragPhase) {
        let _ = self.send_command(GraphCommand::DragNodeTarget { id, target_pos, phase });
    }

    /// Asynchronously load a new graph preset.
    pub fn load_preset(&self, preset_state: GraphState<S>) {
        let _ = self.send_command(GraphCommand::LoadPreset(preset_state));
    }

    /// Asynchronously run a layout algorithm on the background thread.
    pub fn run_layout(&self, layout: LayoutCommand) {
        let _ = self.send_command(GraphCommand::RunLayout(layout));
    }

    /// Asynchronously set expansion mode for a node on the background GraphEngine thread.
    pub fn set_node_expansion_mode(&self, id: NodeId, mode: graphene_core::DataExpansionMode) {
        let _ = self.send_command(GraphCommand::SetNodeExpansionMode { id, mode });
    }

    /// Shutdown the background GraphEngine thread.
    pub fn shutdown(mut self) {
        let _ = self.command_tx.send(GraphCommand::Shutdown);
        if let Some(handle) = self.engine_thread.take() {
            let _ = handle.join();
        }
    }
}
