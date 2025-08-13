use bevy::prelude::*;
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::window::{PresentMode, WindowResolution};
use bevy::pbr::DirectionalLightShadowMap;
use bevy::render::camera::Projection;
use bevy::render::view::Msaa;
use bevy_egui::{egui, EguiContexts, EguiPlugin};
use bevy::asset::AssetServer;
use bevy::diagnostic::DiagnosticsStore;
use crate::localization::LocalizationManager;
use crate::scripting::ScriptEngine;
use crate::block_registry::BlockRegistry;
use crate::game_state::GameState;

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(EguiPlugin)
            .insert_resource(DebugState::default())
            .insert_resource(GameSettings::default())
            .add_systems(PostStartup, apply_initial_settings)
            .add_systems(Update, (
                debug_ui_system.run_if(in_state(GameState::InGame)),
                game_settings_ui.run_if(in_state(GameState::InGame)),
                crosshair_ui.run_if(in_state(GameState::InGame)),
            ));
    }
}

#[derive(Resource, Default)]
pub struct DebugState {
    pub fps: f32,
    pub chunks_loaded: usize,
}

#[derive(Resource, Clone)]
pub struct GameSettings {
    pub show_settings: bool,
    pub msaa_samples: u32,
    pub shadows_enabled: bool,
    pub shadow_resolution: u32,
    pub vsync_enabled: bool,
    pub resolution_width: f32,
    pub resolution_height: f32,
    pub tonemapping_mode: TonemappingMode,
    pub max_loaded_chunks: u32,
    pub surface_priority_quota: u32,
    pub sphere_loading_radius: f32,
    pub fov: f32,
    pub mouse_sensitivity: f32,
    pub gravity: f32,
    pub chunk_generation_threads: u32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TonemappingMode {
    None,
    Reinhard,
    ReinhardLuminance,
    AcesFitted,
    AgX,
    SomewhatBoring,
    TonyMcMapface,
    BlenderFilmic,
}

impl Default for GameSettings {
    fn default() -> Self {
        Self {
            show_settings: false,
            msaa_samples: 4,
            shadows_enabled: true,
            shadow_resolution: 1024,
            vsync_enabled: true,
            resolution_width: 1920.0,
            resolution_height: 1080.0,
            tonemapping_mode: TonemappingMode::Reinhard,
            max_loaded_chunks: 1000,
            surface_priority_quota: 600,
            sphere_loading_radius: 12.0,
            fov: 70.0,
            mouse_sensitivity: 1.0,
            gravity: 9.8,
            chunk_generation_threads: 32,
        }
    }
}

fn debug_ui_system(
    mut contexts: EguiContexts,
    mut state: ResMut<DebugState>,
    diagnostics: Res<DiagnosticsStore>,
    mut game_settings: ResMut<GameSettings>,
    _world_state: Option<Res<crate::world::WorldState>>,
    chunk_storage: Option<Res<crate::world::storage::ChunkStorage>>,
    localization: Res<LocalizationManager>,
    script: Option<Res<ScriptEngine>>,
    registry: Option<Res<BlockRegistry>>,
) {
    if let Some(fps_diagnostic) = diagnostics.get(bevy::diagnostic::FrameTimeDiagnosticsPlugin::FPS) {
        if let Some(fps) = fps_diagnostic.smoothed() { state.fps = fps as f32; }
    }
    
    if let Some(chunk_storage) = chunk_storage {
        state.chunks_loaded = chunk_storage.chunks.len();
    }
    
    let loc = contexts.ctx_mut();
    egui::Window::new(localization.get("game_info")).show(loc, |ui| {
        ui.label(format!("{}: {:.1}", localization.get("fps"), state.fps));
        ui.label(format!("{}: {}", localization.get("chunks_loaded"), state.chunks_loaded));
        if let Some(reg) = registry { ui.label(format!("Script blocks: {}", reg.definitions.len())); }
        ui.separator();
        ui.label(localization.get("controls_hint"));
        ui.separator();
        if ui.button(localization.get("game_settings")).clicked() {
            game_settings.show_settings = !game_settings.show_settings;
        }
        if ui.button("Run Lua hello()").clicked() {
            if let Some(engine) = script {
                if let Err(e) = engine.call0::<()>("hello") {
                    info!("Lua call error: {e}");
                }
            }
        }
    });
}

#[derive(Component)]
struct CrosshairMarker;

fn crosshair_ui(
    mut commands: Commands,
    crosshair_query: Query<Entity, With<CrosshairMarker>>,
) {
    if !crosshair_query.is_empty() {
        return;
    }

    commands
        .spawn((
            NodeBundle {
                style: Style {
                    position_type: PositionType::Absolute,
                    left: Val::Percent(50.0),
                    top: Val::Percent(50.0),
                    margin: UiRect {
                        left: Val::Px(-10.0),
                        top: Val::Px(-1.0),
                        ..default()
                    },
                    width: Val::Px(20.0),
                    height: Val::Px(2.0),
                    ..default()
                },
                background_color: Color::WHITE.into(),
                z_index: ZIndex::Global(1000),
                ..default()
            },
            CrosshairMarker,
        ));

    commands
        .spawn((
            NodeBundle {
                style: Style {
                    position_type: PositionType::Absolute,
                    left: Val::Percent(50.0),
                    top: Val::Percent(50.0),
                    margin: UiRect {
                        left: Val::Px(-1.0),
                        top: Val::Px(-10.0),
                        ..default()
                    },
                    width: Val::Px(2.0),
                    height: Val::Px(20.0),
                    ..default()
                },
                background_color: Color::WHITE.into(),
                z_index: ZIndex::Global(1000),
                ..default()
            },
            CrosshairMarker,
        ));
}

fn game_settings_ui(
    mut contexts: EguiContexts,
    mut game_settings: ResMut<GameSettings>,
    mut msaa: ResMut<Msaa>,
    mut windows: Query<&mut Window>,
    mut projection_query: Query<&mut Projection>,
    mut tonemapping_query: Query<&mut Tonemapping>,
    mut light_query: Query<&mut DirectionalLight>,
    mut shadow_map: ResMut<DirectionalLightShadowMap>,
    localization: Res<LocalizationManager>,
) {
    if !game_settings.show_settings {
        return;
    }

    let ctx = contexts.ctx_mut();

    egui::Window::new(localization.get("game_settings"))
        .collapsible(false)
        .resizable(false)
        .show(ctx, |ui| {
            // MSAA
            ui.horizontal(|ui| {
                ui.label(localization.get("msaa"));
                let mut msaa_changed = false;
                egui::ComboBox::from_id_source("msaa")
                    .selected_text(
                        if game_settings.msaa_samples == 1 {
                            localization.get("off")
                        } else {
                            "Multi"
                        }
                    )
                    .show_ui(ui, |ui| {
                        if ui.selectable_value(&mut game_settings.msaa_samples, 1, localization.get("off")).clicked() {
                            msaa_changed = true;
                        }
                        if ui.selectable_value(&mut game_settings.msaa_samples, 2, "2x").clicked() {
                            msaa_changed = true;
                        }
                        if ui.selectable_value(&mut game_settings.msaa_samples, 4, "4x").clicked() {
                            msaa_changed = true;
                        }
                        if ui.selectable_value(&mut game_settings.msaa_samples, 8, "8x").clicked() {
                            msaa_changed = true;
                        }
                    });
                
                if msaa_changed {
                    *msaa = match game_settings.msaa_samples {
                        1 => Msaa::Off,
                        2 => Msaa::Sample2,
                        4 => Msaa::Sample4,
                        8 => Msaa::Sample8,
                        _ => Msaa::Sample4,
                    };
                }
            });

            // Shadows
            if ui.checkbox(&mut game_settings.shadows_enabled, localization.get("enable_shadows")).changed() {
                for mut light in light_query.iter_mut() {
                    light.shadows_enabled = game_settings.shadows_enabled;
                }
            }

            if game_settings.shadows_enabled {
                ui.horizontal(|ui| {
                    ui.label(localization.get("shadow_resolution"));
                    let mut shadow_changed = false;
                    egui::ComboBox::from_id_source("shadow_resolution")
                        .selected_text(format!("{}x{}", game_settings.shadow_resolution, game_settings.shadow_resolution))
                        .show_ui(ui, |ui| {
                            if ui.selectable_value(&mut game_settings.shadow_resolution, 512, "512x512").clicked() {
                                shadow_changed = true;
                            }
                            if ui.selectable_value(&mut game_settings.shadow_resolution, 1024, "1024x1024").clicked() {
                                shadow_changed = true;
                            }
                            if ui.selectable_value(&mut game_settings.shadow_resolution, 2048, "2048x2048").clicked() {
                                shadow_changed = true;
                            }
                            if ui.selectable_value(&mut game_settings.shadow_resolution, 4096, "4096x4096").clicked() {
                                shadow_changed = true;
                            }
                        });
                    
                    if shadow_changed {
                        shadow_map.size = game_settings.shadow_resolution as usize;
                    }
                });
            }

            // VSync
            if ui.checkbox(&mut game_settings.vsync_enabled, localization.get("vsync")).changed() {
                if let Ok(mut window) = windows.get_single_mut() {
                    window.present_mode = if game_settings.vsync_enabled {
                        PresentMode::AutoVsync
                    } else {
                        PresentMode::AutoNoVsync
                    };
                }
            }

            // FOV
            ui.horizontal(|ui| {
                ui.label(localization.get("fov"));
                if ui.add(egui::Slider::new(&mut game_settings.fov, 60.0..=120.0).text("°")).changed() {
                    for mut proj in projection_query.iter_mut() {
                        if let Projection::Perspective(ref mut persp) = *proj {
                            persp.fov = game_settings.fov.to_radians();
                        }
                    }
                }
            });

            // Mouse Sensitivity
            ui.horizontal(|ui| {
                ui.label(localization.get("mouse_sensitivity"));
                ui.add(egui::Slider::new(&mut game_settings.mouse_sensitivity, 0.1..=3.0).step_by(0.1));
            });

            // Gravity
            ui.horizontal(|ui| {
                ui.label(localization.get("gravity"));
                ui.add(egui::Slider::new(&mut game_settings.gravity, 1.0..=20.0).step_by(0.1));
            });

            // Max Loaded Chunks
            ui.horizontal(|ui| {
                ui.label(localization.get("max_loaded_chunks"));
                let mut chunk_text = game_settings.max_loaded_chunks.to_string();
                let response = ui.text_edit_singleline(&mut chunk_text);
                if response.changed() {
                    if let Ok(value) = chunk_text.parse::<u32>() {
                        if value >= 100 && value <= 10000 {
                            game_settings.max_loaded_chunks = value;
                        }
                    }
                }
            });
            ui.colored_label(egui::Color32::GRAY, localization.get("max_chunks_hint"));

            // Surface Priority Quota
            ui.horizontal(|ui| {
                ui.label(localization.get("surface_priority_quota"));
                let mut quota_text = game_settings.surface_priority_quota.to_string();
                let response = ui.text_edit_singleline(&mut quota_text);
                if response.changed() {
                    if let Ok(value) = quota_text.parse::<u32>() {
                        if value <= game_settings.max_loaded_chunks {
                            game_settings.surface_priority_quota = value;
                        }
                    }
                }
            });
            ui.colored_label(egui::Color32::GRAY, localization.get("surface_quota_hint"));

            // Sphere Loading Radius
            ui.horizontal(|ui| {
                ui.label(localization.get("sphere_loading_radius"));
                ui.add(egui::Slider::new(&mut game_settings.sphere_loading_radius, 5.0..=25.0).text("chunks"));
            });

            // Chunk Generation Threads
            ui.horizontal(|ui| {
                ui.label(localization.get("chunk_generation_threads"));
                let mut thread_text = game_settings.chunk_generation_threads.to_string();
                let response = ui.text_edit_singleline(&mut thread_text);
                if response.changed() {
                    if let Ok(value) = thread_text.parse::<u32>() {
                        if value > 0 {
                            game_settings.chunk_generation_threads = value;
                        }
                    }
                }
            });
            ui.colored_label(egui::Color32::YELLOW, localization.get("chunk_threads_warning"));

            // Tonemapping
            ui.collapsing(localization.get("tonemapping"), |ui| {
                ui.horizontal(|ui| {
                    ui.label(localization.get("tonemapping_mode"));
                    let mut tone_changed = false;
                    egui::ComboBox::from_id_source("tonemapping_mode")
                        .selected_text(match game_settings.tonemapping_mode {
                            TonemappingMode::None => localization.get("none"),
                            TonemappingMode::Reinhard => localization.get("reinhard"),
                            TonemappingMode::ReinhardLuminance => localization.get("reinhard_luminance"),
                            TonemappingMode::AcesFitted => localization.get("aces_fitted"),
                            TonemappingMode::AgX => localization.get("agx"),
                            TonemappingMode::SomewhatBoring => localization.get("somewhat_boring"),
                            TonemappingMode::TonyMcMapface => localization.get("tony_mc_mapface"),
                            TonemappingMode::BlenderFilmic => localization.get("blender_filmic"),
                        })
                        .show_ui(ui, |ui| {
                            if ui.selectable_value(&mut game_settings.tonemapping_mode, TonemappingMode::None, localization.get("none")).clicked() {
                                tone_changed = true;
                            }
                            if ui.selectable_value(&mut game_settings.tonemapping_mode, TonemappingMode::Reinhard, localization.get("reinhard")).clicked() {
                                tone_changed = true;
                            }
                            if ui.selectable_value(&mut game_settings.tonemapping_mode, TonemappingMode::ReinhardLuminance, localization.get("reinhard_luminance")).clicked() {
                                tone_changed = true;
                            }
                            if ui.selectable_value(&mut game_settings.tonemapping_mode, TonemappingMode::AcesFitted, localization.get("aces_fitted")).clicked() {
                                tone_changed = true;
                            }
                            if ui.selectable_value(&mut game_settings.tonemapping_mode, TonemappingMode::AgX, localization.get("agx")).clicked() {
                                tone_changed = true;
                            }
                            if ui.selectable_value(&mut game_settings.tonemapping_mode, TonemappingMode::SomewhatBoring, localization.get("somewhat_boring")).clicked() {
                                tone_changed = true;
                            }
                            if ui.selectable_value(&mut game_settings.tonemapping_mode, TonemappingMode::TonyMcMapface, localization.get("tony_mc_mapface")).clicked() {
                                tone_changed = true;
                            }
                            if ui.selectable_value(&mut game_settings.tonemapping_mode, TonemappingMode::BlenderFilmic, localization.get("blender_filmic")).clicked() {
                                tone_changed = true;
                            }
                        });
                    
                    if tone_changed {
                        let mapped = match game_settings.tonemapping_mode {
                            TonemappingMode::None => Tonemapping::None,
                            TonemappingMode::Reinhard => Tonemapping::Reinhard,
                            TonemappingMode::ReinhardLuminance => Tonemapping::ReinhardLuminance,
                            TonemappingMode::AcesFitted => Tonemapping::AcesFitted,
                            TonemappingMode::AgX => Tonemapping::AgX,
                            TonemappingMode::SomewhatBoring => Tonemapping::SomewhatBoringDisplayTransform,
                            TonemappingMode::TonyMcMapface => Tonemapping::TonyMcMapface,
                            TonemappingMode::BlenderFilmic => Tonemapping::BlenderFilmic,
                        };
                        for mut t in tonemapping_query.iter_mut() {
                            *t = mapped;
                        }
                    }
                });
            });

            ui.horizontal(|ui| {
                if ui.button(localization.get("close")).clicked() {
                    game_settings.show_settings = false;
                }

                if ui.button(localization.get("restore_defaults")).clicked() {
                    let old_show = game_settings.show_settings;
                    *game_settings = GameSettings::default();
                    game_settings.show_settings = old_show;
                    
                    // Apply default settings
                    *msaa = Msaa::Sample4;
                    for mut light in light_query.iter_mut() {
                        light.shadows_enabled = true;
                    }
                    shadow_map.size = 1024;
                    for mut t in tonemapping_query.iter_mut() {
                        *t = Tonemapping::Reinhard;
                    }
                    if let Ok(mut window) = windows.get_single_mut() {
                        window.present_mode = PresentMode::AutoVsync;
                        window.resolution = WindowResolution::new(1920.0, 1080.0);
                    }
                    for mut proj in projection_query.iter_mut() {
                        if let Projection::Perspective(ref mut persp) = *proj {
                            persp.far = 1000.0;
                            persp.fov = 70.0_f32.to_radians();
                        }
                    }
                }
            });
        });
}

fn apply_initial_settings(
    mut msaa: ResMut<Msaa>,
    mut windows: Query<&mut Window>,
    mut projection_query: Query<&mut Projection>,
    mut tonemapping_query: Query<&mut Tonemapping>,
    mut light_query: Query<&mut DirectionalLight>,
    mut shadow_map: ResMut<DirectionalLightShadowMap>,
    game_settings: Res<GameSettings>,
    thread_pool: Option<ResMut<crate::world::chunk_loader::ChunkGenerationThreadPool>>,
) {
    // Apply MSAA
    *msaa = match game_settings.msaa_samples {
        1 => Msaa::Off,
        2 => Msaa::Sample2,
        4 => Msaa::Sample4,
        8 => Msaa::Sample8,
        _ => Msaa::Sample4,
    };

    // Apply shadows
    for mut light in light_query.iter_mut() {
        light.shadows_enabled = game_settings.shadows_enabled;
    }
    shadow_map.size = game_settings.shadow_resolution as usize;

    // Apply VSync and resolution
    if let Ok(mut window) = windows.get_single_mut() {
        window.present_mode = if game_settings.vsync_enabled {
            PresentMode::AutoVsync
        } else {
            PresentMode::AutoNoVsync
        };
        window.resolution = WindowResolution::new(
            game_settings.resolution_width,
            game_settings.resolution_height,
        );
    }

    // Apply FOV
    for mut proj in projection_query.iter_mut() {
        if let Projection::Perspective(ref mut persp) = *proj {
            persp.fov = game_settings.fov.to_radians();
        }
    }

    // Apply tonemapping
    let mapped = match game_settings.tonemapping_mode {
        TonemappingMode::None => Tonemapping::None,
        TonemappingMode::Reinhard => Tonemapping::Reinhard,
        TonemappingMode::ReinhardLuminance => Tonemapping::ReinhardLuminance,
        TonemappingMode::AcesFitted => Tonemapping::AcesFitted,
        TonemappingMode::AgX => Tonemapping::AgX,
        TonemappingMode::SomewhatBoring => Tonemapping::SomewhatBoringDisplayTransform,
        TonemappingMode::TonyMcMapface => Tonemapping::TonyMcMapface,
        TonemappingMode::BlenderFilmic => Tonemapping::BlenderFilmic,
    };
    for mut t in tonemapping_query.iter_mut() {
        *t = mapped;
    }

    // Apply chunk generation thread pool settings
    if let Some(mut pool) = thread_pool {
        pool.update_thread_count(game_settings.chunk_generation_threads);
    }
}