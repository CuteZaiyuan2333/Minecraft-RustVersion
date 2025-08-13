use bevy::prelude::*;
use bevy_egui::{EguiContexts, EguiPlugin, egui};
use noise_engine::{graph::Graph};
mod ui_strings; mod graph_editor; mod preview;
use ui_strings::UiStrings;

struct EditorState {
    graph: Graph,
    engine: noise_engine::sampling::SimpleEngine,
    seed: u64,
    selected_channels: Vec<String>,
    ui: UiStrings,
    preview_w: u32,
    preview_h: u32,
    preview_channel: usize,
    show_preview_window: bool,
    preview_window_entity: Option<Entity>,
}

impl Default for EditorState { fn default() -> Self { let graph = Graph { nodes: vec![], edges: vec![] }; Self { engine: noise_engine::sampling::SimpleEngine::new(graph.clone()), graph, seed: 0, selected_channels: vec![], ui: UiStrings::default(), preview_w: 256, preview_h: 256, preview_channel: 0, show_preview_window: false, preview_window_entity: None } } }

fn setup(mut commands: Commands) { commands.spawn(Camera2dBundle::default()); }

fn ui_system(mut contexts: EguiContexts, mut state: ResMut<EditorState>) {
    let ctx = contexts.ctx_mut();
    egui::CentralPanel::default().show(ctx, |ui| {
        ui.heading(&state.ui.graph_panel.title);
        let mut graph_ref = &mut state.graph;
        let ui_clone = state.ui.clone();
        graph_editor::graph_editor_ui(ui, &mut graph_ref, &ui_clone);
    });
}

fn main() {
    App::new()
        .insert_resource(EditorState::default())
        .add_plugins((DefaultPlugins, EguiPlugin))
        .add_systems(Startup, setup)
        .add_systems(Update, ui_system)
        .run();
}
