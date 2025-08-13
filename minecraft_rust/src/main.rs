use bevy::prelude::*;
use bevy::diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin};
use bevy::asset::AssetPlugin;
use bevy::render::texture::ImagePlugin;
use std::env;

mod systems;
mod world;
mod rendering;
mod ui;
mod localization;
mod scripting;
mod block_registry;
mod controller;
mod inventory;
mod hud;
mod game_state;
// 菜单模块已移除，所有菜单功能在启动器中实现
// mod main_menu;
// mod pause_menu;
// mod settings_menu;
mod ui_strings;

use crate::localization::{LocalizationManager, LanguageChangeEvent, handle_language_change};
use crate::scripting::ScriptEngine;
use crate::block_registry::BlockRegistry;
use crate::controller::{ControllerPlugin, FirstPersonController};
use crate::world::generator::{WorldGenerator, WorldGeneratorConfig};
use crate::game_state::{GameState, GameStatePlugin};
use crate::ui_strings::UiStringManager;

// 启动参数资源已移除，游戏直接启动到游戏状态

fn setup_scripting(engine: Res<ScriptEngine>, mut registry: ResMut<BlockRegistry>) {
    // Try load all scripts at startup, ignore errors but log
    if let Err(e) = engine.load_all() {
        error!("Failed to load Lua scripts: {e}");
    } else {
        info!("Lua scripts loaded successfully");
    }
    if let Err(e) = registry.load_from_scripts(&engine) {
        warn!("Failed to load blocks from scripts: {e}");
    }
}

fn find_safe_spawn_point(generator: &WorldGenerator) -> (i32, i32, i32) {
    // 尝试在原点附近找到一个安全的出生点
    for radius in 0..10 {
        for x in -radius..=radius {
            for z in -radius..=radius {
                // 只检查圆形边界上的点
                if x*x + z*z == radius*radius || radius == 0 {
                    let surface_height = generator.get_surface_height(x, z);
                    
                    // 确保出生点不在水下（假设海平面是64）
                    if surface_height >= 64 {
                        return (x, surface_height, z);
                    }
                }
            }
        }
    }
    
    // 如果找不到合适的点，就使用原点
    (0, generator.get_surface_height(0, 0), 0)
}

// 主菜单摄像头设置函数已移除

fn setup_game_camera(
    mut commands: Commands,
) {
    // 创建世界生成器来计算地面高度
    let config = WorldGeneratorConfig::default();
    let generator = WorldGenerator::new(config);
    
    // 找到安全的出生点
    let (spawn_x, surface_height, spawn_z) = find_safe_spawn_point(&generator);
    let spawn_y = surface_height as f32 + 3.0; // 在地面上方3格出生
    
    info!("Player spawning at surface height: {} (world pos: {}, {}, {})", surface_height, spawn_x, spawn_y, spawn_z);

    let player = commands.spawn((
        SpatialBundle {
            transform: Transform::from_xyz(spawn_x as f32, spawn_y, spawn_z as f32),
            ..default()
        },
        FirstPersonController::default(),
        inventory::PlayerInventory::new(),
    )).id();

    commands.entity(player).with_children(|parent| {
        parent.spawn(Camera3dBundle {
            transform: Transform::from_xyz(0.0, 1.62, 0.0),
            ..default()
        });
    });
}

fn setup_localization(mut commands: Commands) {
    info!("Initializing localization system...");
    let mut localization = LocalizationManager::new();
    localization.scan_languages();
    if let Err(e) = localization.load_language("en_us") {
        warn!("Failed to load default language: {}", e);
    }
    commands.insert_resource(localization);
}

// 启动参数解析和初始状态设置函数已移除

fn setup_initial_state(mut next_state: ResMut<NextState<GameState>>) {
    info!("Starting game directly without menu");
    next_state.set(GameState::InGame);
}

fn main() {
    App::new()
        .add_event::<LanguageChangeEvent>()
        .insert_resource(ClearColor(Color::rgb(0.53, 0.81, 0.92)))
        .insert_resource(ScriptEngine::default())
        .insert_resource(BlockRegistry::default())
        .insert_resource(UiStringManager::new())
        .add_plugins(DefaultPlugins
            .set(WindowPlugin {
                primary_window: Some(Window {
                    title: "Minecraft Rust".into(),
                    resolution: (1280.0, 720.0).into(),
                    resizable: true,
                    ..default()
                }),
                ..default()
            })
            .set(AssetPlugin {
                file_path: "g:/Document/MinecarftRustver/minecraft".to_string(),
                ..default()
            })
            .set(ImagePlugin::default_nearest())
        )
        .add_plugins(LogDiagnosticsPlugin::default())
        .add_plugins(FrameTimeDiagnosticsPlugin::default())
        // 游戏状态管理
        .add_plugins(GameStatePlugin)
        // UI插件（仅保留游戏内UI）
        .add_plugins(ui::UiPlugin)
        // 游戏系统插件
        .add_plugins(world::WorldPlugin)
        .add_plugins(rendering::RenderingPlugin)
        .add_plugins(controller::ControllerPlugin)
        .add_plugins(inventory::InventoryPlugin)
        .add_plugins(hud::HudPlugin)
        // 启动系统
        .add_systems(Startup, (setup_localization, setup_scripting, setup_initial_state).chain())
        .add_systems(OnEnter(GameState::InGame), setup_game_camera)
        // 本地化系统
        .add_systems(Update, handle_language_change)
        .run();
}