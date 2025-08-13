use bevy_egui::egui;
use noise_engine::graph::Graph;
use crate::ui_strings::UiStrings;

pub fn graph_editor_ui(ui: &mut egui::Ui, graph: &mut Graph, strings: &UiStrings) {
    // Toolbar
    ui.horizontal(|ui| {
        if ui.button(&strings.graph_panel.add_node).clicked() {
            let id = (graph.nodes.len() as u64) + 1;
            let name = format!("{} {}", strings.graph_panel.node_prefix, id);
            graph.nodes.push(noise_engine::graph::Node { id, name, kind: noise_engine::graph::NodeKind::Constant(0.0) });
        }
        if ui.button(&strings.graph_panel.clear).clicked() {
            graph.nodes.clear();
            graph.edges.clear();
        }
        ui.separator();
        ui.label(&strings.graph_panel.hint);
    });

    ui.separator();

    // Simple fallback list-based editor (no hardcoded UI strings beyond data)
    // Nodes list
    for n in &graph.nodes {
        ui.label(&n.name);
    }

    // Show edges as pairs of ids (data only)
    for e in &graph.edges {
        // Represent as "from -> to" using numeric ids; this is data, not UI text
        let txt = format!("{} -> {}", e.from, e.to);
        ui.label(txt);
    }
}