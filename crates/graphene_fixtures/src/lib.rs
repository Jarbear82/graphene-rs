pub mod advanced;
pub mod basic;
pub mod demos;

use graphene_core::{EdgeData, GraphState, NodeId, Size2, Vec2};
use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct GraphFixture<S: Copy + Default> {
    pub name: String,
    pub description: String,
    pub state: GraphState<S>,
    pub weights: HashMap<usize, f32>, // edge_idx -> weight
    pub node_labels: HashMap<NodeId, String>,
    pub edge_labels: HashMap<usize, String>,
    pub node_attributes: HashMap<NodeId, HashMap<String, String>>,
    pub edge_attributes: HashMap<usize, HashMap<String, String>>,
    pub is_directed: bool,
    pub compound_groups: HashMap<NodeId, Vec<NodeId>>, // parent -> children
    pub hyperedges: Vec<Vec<NodeId>>,
    pub chart_data: HashMap<NodeId, HashMap<String, f32>>, // node -> {metric: value}
}

impl<S: Copy + Default> GraphFixture<S> {
    pub fn new(name: &str, description: &str) -> Self {
        Self {
            name: name.to_string(),
            description: description.to_string(),
            state: GraphState::new(),
            weights: HashMap::new(),
            node_labels: HashMap::new(),
            edge_labels: HashMap::new(),
            node_attributes: HashMap::new(),
            edge_attributes: HashMap::new(),
            is_directed: true,
            compound_groups: HashMap::new(),
            hyperedges: Vec::new(),
            chart_data: HashMap::new(),
        }
    }
}

pub(crate) fn add_dir_to_fixture<S: Copy + Default>(
    f: &mut GraphFixture<S>,
    dir_path: &std::path::Path,
    parent_id: Option<NodeId>,
    depth_limit: usize,
) {
    if depth_limit == 0 {
        return;
    }

    let dir_name = dir_path
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "Root".to_string());

    let dir_node = f
        .state
        .add_node(Vec2::new(0.0, 0.0), Size2::new(50.0, 30.0));
    f.node_labels.insert(dir_node, dir_name);

    if let Some(pid) = parent_id {
        f.state.reparent_node(dir_node, Some(pid));
        f.state.add_edge(pid, dir_node, EdgeData::default());
    }

    if let Ok(entries) = std::fs::read_dir(dir_path) {
        let mut entry_paths = Vec::new();
        for entry in entries.filter_map(Result::ok) {
            entry_paths.push(entry.path());
        }
        entry_paths.sort();

        for path in entry_paths {
            let name = path
                .file_name()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_default();

            if path.is_dir() {
                if !name.starts_with('.') && name != "target" {
                    add_dir_to_fixture(f, &path, Some(dir_node), depth_limit - 1);
                }
            } else {
                let file_node = f
                    .state
                    .add_node(Vec2::new(0.0, 0.0), Size2::new(40.0, 30.0));
                f.node_labels.insert(file_node, name);
                f.state.reparent_node(file_node, Some(dir_node));
                f.state.add_edge(dir_node, file_node, EdgeData::default());
            }
        }
    }
}

impl<S: Copy + Default> Default for GraphFixture<S> {
    fn default() -> Self {
        Self::new("Default Fixture", "Default uninitialized graph fixture")
    }
}

pub fn get_all_fixtures<S: Copy + Default>() -> Vec<GraphFixture<S>> {
    let mut fixtures = Vec::new();
    basic::add_basic_fixtures(&mut fixtures);
    advanced::add_advanced_fixtures(&mut fixtures);
    demos::add_cytoscape_demos(&mut fixtures);
    fixtures
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_all_fixtures_non_empty() {
        let fixtures = get_all_fixtures::<()>();
        assert!(!fixtures.is_empty(), "Fixtures vector should not be empty");
        for f in &fixtures {
            assert!(!f.name.is_empty());
        }
    }

    #[test]
    fn test_fixture_default() {
        let f = GraphFixture::<()>::default();
        assert_eq!(f.name, "Default Fixture");
    }
}
