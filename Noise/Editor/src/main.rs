use bevy::{prelude::*, window::WindowResolution};
use bevy_egui::{egui, EguiContexts, EguiPlugin};
use bevy::render::camera::RenderTarget;
use bevy::window::WindowRef;
use bevy::math::primitives::Cuboid;
use noise_engine::*; // API types
use noise_engine::graph::Graph; // graph types
use noise_engine::sampling::SimpleEngine; // engine impl

mod ui_strings;
mod preview;
mod graph_editor;

#[derive(Resource)]
struct EditorState {
    graph: Graph,
    engine: Option<SimpleEngine>,
    seed: u64,
    selected_channels: Vec<ChannelDesc>,
    ui: ui_strings::UiStrings,
    preview_w: i32,
    preview_h: i32,
    preview_channel: i32,
    show_preview_window: bool,
    preview_window_entity: Option<Entity>,
}

impl Default for EditorState {
    fn default() -> Self {
        Self {
            graph: Graph { nodes: vec![], edges: vec![] },
            engine: None,
            seed: 1337,
            selected_channels: vec![],
            ui: ui_strings::UiStrings::default(),
            preview_w: 256,
            preview_h: 256,
            preview_channel: 0,
            show_preview_window: false,
            preview_window_entity: None,
        }
    }
}

const DEFAULT_GRAPH_PATH: &str = "assets/noise_graphs/default.ron";

fn main() {
    // Load UI strings early to get window title
    let ui_strings = ui_strings::load_from_file("assets/ui_strings.json").unwrap_or_default();

    App::new()
        .insert_resource(ClearColor(Color::rgb(0.05, 0.05, 0.08)))
        .add_plugins((DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window { title: ui_strings.app.window_title.clone().into(), resolution: WindowResolution::new(1280., 800.), ..default() }),
            ..default()
        }), EguiPlugin))
        .insert_resource(EditorState { ui: ui_strings, ..Default::default() })
        .add_systems(Startup, setup)
        .add_systems(Update, (draw_menu, draw_left_panel, draw_preview, spawn_preview_world_window, monitor_preview_window_closed))
        .run();
}

fn setup(mut state: ResMut<EditorState>) {
    state.graph = Graph { nodes: vec![], edges: vec![] };
    state.seed = 1337;
    state.selected_channels = vec![
        ChannelDesc { name: "height".into(), kind: ChannelKind::Height2D },
        ChannelDesc { name: "biome".into(), kind: ChannelKind::Biome2D },
    ];
    // Try load default graph
    if let Ok(s) = std::fs::read_to_string(DEFAULT_GRAPH_PATH) {
        if let Ok(g) = ron::from_str::<Graph>(&s) { state.graph = g; }
    }
    state.engine = Some(SimpleEngine::new(state.graph.clone()));
}

fn draw_menu(mut egui_ctx: EguiContexts, mut state: ResMut<EditorState>) {
    egui::TopBottomPanel::top("menu_bar").show(egui_ctx.ctx_mut(), |ui| {
        egui::menu::bar(ui, |ui| {
            let ui_text = state.ui.clone();
            let file_text = ui_text.menu.file.clone();
            ui.menu_button(file_text, move |ui| {
                if ui.button(&ui_text.menu.save).clicked() {
                    let _ = std::fs::create_dir_all("assets/noise_graphs");
                    ui.data_mut(|d| d.insert_temp(egui::Id::new("do_save_graph"), true));
                    ui.close_menu();
                }
                if ui.button(&ui_text.menu.load).clicked() {
                    ui.data_mut(|d| d.insert_temp(egui::Id::new("do_load_graph"), true));
                    ui.close_menu();
                }
            });
            let bake_text = ui_text.menu.bake.clone();
            if ui.button(&bake_text).clicked() {
                let seed = state.seed; // take value to avoid immutable borrow later
                if let Some(engine) = &mut state.engine { engine.bake(Seed(seed)); }
            }
        });
    });
    // Handle deferred actions that require mutable access to state outside of the UI borrow scope
    let ctx = egui_ctx.ctx_mut();
    let do_save = ctx.data_mut(|d| d.get_temp::<bool>(egui::Id::new("do_save_graph")).unwrap_or(false));
    if do_save {
        let _ = std::fs::create_dir_all("assets/noise_graphs");
        let pretty = ron::ser::PrettyConfig::new();
        if let Ok(text) = ron::ser::to_string_pretty(&state.graph, pretty) {
            let _ = std::fs::write(DEFAULT_GRAPH_PATH, text);
        }
        ctx.data_mut(|d| d.remove::<bool>(egui::Id::new("do_save_graph")));
    }
    let do_load = ctx.data_mut(|d| d.get_temp::<bool>(egui::Id::new("do_load_graph")).unwrap_or(false));
    if do_load {
        if let Ok(s) = std::fs::read_to_string(DEFAULT_GRAPH_PATH) {
            if let Ok(g) = ron::from_str::<Graph>(&s) {
                state.graph = g;
                let graph_clone = state.graph.clone();
                if let Some(engine) = &mut state.engine { engine.graph = graph_clone; }
            }
        }
        ctx.data_mut(|d| d.remove::<bool>(egui::Id::new("do_load_graph")));
    }
}

fn draw_left_panel(mut egui_ctx: EguiContexts, mut state: ResMut<EditorState>) {
    egui::SidePanel::left("graph_panel")
        .resizable(true)
        .default_width(400.0)
        .min_width(200.0)
        .max_width(600.0)
        .show(egui_ctx.ctx_mut(), |ui| {
            // Render headings using an immutable snapshot of UI strings to avoid borrow conflicts
            let ui_clone = state.ui.clone();
            ui.heading(&ui_clone.graph_panel.title);
            ui.label(&ui_clone.graph_panel.hint);
            // Limit the lifetime of the mutable borrow of graph to this block
            {
                let graph_ref = &mut state.graph;
                graph_editor::graph_editor_ui(ui, graph_ref, &ui_clone);
            }
            // Clone graph before mutably borrowing engine to avoid E0502
            let graph_clone = state.graph.clone();
            if let Some(engine) = &mut state.engine { engine.graph = graph_clone; }
        });
}

fn draw_preview(mut egui_ctx: EguiContexts, mut state: ResMut<EditorState>) {
    egui::CentralPanel::default().show(egui_ctx.ctx_mut(), |ui| {
        let ui_text = state.ui.clone();
        preview::preview_ui(ui, &mut state, &ui_text);
    });
}

fn spawn_preview_world_window(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut state: ResMut<EditorState>,
) {
    if state.show_preview_window && state.preview_window_entity.is_none() {
        // Spawn secondary OS window
        let title = state.ui.preview.window_title.clone();
        let window_entity = commands
            .spawn(Window {
                title: title.into(),
                resolution: WindowResolution::new(960., 640.),
                ..default()
            })
            .id();

        // Choose color based on selected channel
        let color = match state.preview_channel {
            0 => Color::rgb(1.0, 0.2, 0.2), // R
            1 => Color::rgb(0.2, 1.0, 0.2), // G
            2 => Color::rgb(0.2, 0.2, 1.0), // B
            _ => Color::rgb(0.8, 0.8, 0.8),
        };

        // Camera targeting the new window
        commands.spawn((
            Camera3d::default(),
            Transform::from_xyz(20.0, 20.0, 20.0).looking_at(Vec3::ZERO, Vec3::Y),
            Camera { target: RenderTarget::Window(WindowRef::Entity(window_entity)), ..default() },
        ));

        // Light
        commands.spawn((
            DirectionalLight::default(),
            Transform::from_xyz(30.0, 50.0, 30.0).looking_at(Vec3::ZERO, Vec3::Y),
        ));

        // Ground of monochrome cubes (a small grid)
        let mesh_handle = meshes.add(Mesh::from(Cuboid::new(1.0, 1.0, 1.0)));
        let material_handle = materials.add(StandardMaterial { base_color: color, ..default() });
        let size = 16;
        for x in 0..size {
            for z in 0..size {
                commands.spawn(PbrBundle {
                    mesh: mesh_handle.clone(),
                    material: material_handle.clone(),
                    transform: Transform::from_xyz((x as f32) - size as f32 * 0.5, 0.0, (z as f32) - size as f32 * 0.5),
                    ..default()
                });
            }
        }

        state.preview_window_entity = Some(window_entity);
    }
}

fn monitor_preview_window_closed(
    windows: Query<(), With<Window>>,
    mut state: ResMut<EditorState>,
) {
    if let Some(entity) = state.preview_window_entity {
        if windows.get(entity).is_err() {
            state.preview_window_entity = None;
            state.show_preview_window = false;
        }
    }
}