use bevy_egui::egui;
use noise_engine::graph::Graph;
use crate::ui_strings::UiStrings;

pub fn graph_editor_ui(ui: &mut egui::Ui, graph: &mut Graph, strings: &UiStrings) {
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

    for n in &graph.nodes { ui.label(&n.name); }
    for e in &graph.edges { let txt = format!("{} -> {}", e.from, e.to); ui.label(txt); }
}
