use bevy::prelude::*;
use serde::{Deserialize, Serialize};


/// UI字符串配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiStrings {
    pub launcher: LauncherStrings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LauncherStrings {
    pub title: String,
    pub singleplayer: String,
    pub settings: String,
    pub quit: String,
    pub select_world: String,
    pub back: String,
    pub create_world: String,
    pub settings_title: String,
    pub settings_placeholder: String,
    pub world_examples: WorldExamples,
    pub launch_game: String,
    pub game_started: String,
    pub launch_failed: String,
    pub create_world_todo: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldExamples {
    pub my_world: String,
    pub survival_world: String,
}

/// 启动器状态
#[derive(States, Debug, Clone, PartialEq, Eq, Hash, Default)]
pub enum LauncherState {
    #[default]
    MainMenu,
    WorldSelection,
    Settings,
}

/// 世界信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldInfo {
    pub name: String,
    pub game_mode: String,
    pub world_type: String,
    pub last_played: String,
}

/// 启动器资源
#[derive(Resource, Default)]
pub struct LauncherData {
    pub worlds: Vec<WorldInfo>,
    pub selected_world: Option<String>,
}

/// UI字符串资源
#[derive(Resource)]
pub struct UiStringResource {
    pub strings: UiStrings,
}

/// UI标记组件
#[derive(Component)]
pub struct LauncherUI;

#[derive(Component)]
pub struct WorldButton(pub String);

fn main() {
    // 加载UI字符串
    let ui_strings = load_ui_strings();
    
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: ui_strings.launcher.title.clone(),
                resolution: (800.0, 600.0).into(),
                resizable: false,
                ..default()
            }),
            ..default()
        }))
        .init_state::<LauncherState>()
        .init_resource::<LauncherData>()
        .insert_resource(UiStringResource { strings: ui_strings })
        .add_systems(Startup, setup_launcher)
        .add_systems(OnEnter(LauncherState::MainMenu), setup_main_menu)
        .add_systems(OnEnter(LauncherState::WorldSelection), setup_world_selection)
        .add_systems(OnEnter(LauncherState::Settings), setup_settings)
        .add_systems(OnExit(LauncherState::MainMenu), cleanup_ui)
        .add_systems(OnExit(LauncherState::WorldSelection), cleanup_ui)
        .add_systems(OnExit(LauncherState::Settings), cleanup_ui)
        .add_systems(Update, (
            main_menu_system.run_if(in_state(LauncherState::MainMenu)),
            world_selection_system.run_if(in_state(LauncherState::WorldSelection)),
            settings_system.run_if(in_state(LauncherState::Settings)),
        ))
        .run();
}

fn setup_launcher(mut commands: Commands, mut launcher_data: ResMut<LauncherData>) {
    // 添加UI摄像机
    commands.spawn(Camera2dBundle::default());
    
    // 加载世界列表
    launcher_data.worlds = load_worlds();
}

fn setup_main_menu(mut commands: Commands, ui_strings: Res<UiStringResource>) {
    commands.spawn((
        NodeBundle {
            style: Style {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                row_gap: Val::Px(20.0),
                ..default()
            },
            background_color: Color::srgba(0.1, 0.1, 0.1, 0.95).into(),
            ..default()
        },
        LauncherUI,
    )).with_children(|parent| {
        // 标题
        parent.spawn(TextBundle::from_section(
            &ui_strings.strings.launcher.title,
            TextStyle {
                font: default(),
                font_size: 36.0,
                color: Color::WHITE,
            },
        ));

        // 按钮容器
        parent.spawn(NodeBundle {
            style: Style {
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                row_gap: Val::Px(15.0),
                ..default()
            },
            ..default()
        }).with_children(|parent| {
            create_launcher_button(parent, &ui_strings.strings.launcher.singleplayer, "singleplayer");
            create_launcher_button(parent, &ui_strings.strings.launcher.settings, "settings");
            create_launcher_button(parent, &ui_strings.strings.launcher.quit, "quit");
        });
    });
}

fn setup_world_selection(mut commands: Commands, launcher_data: Res<LauncherData>, ui_strings: Res<UiStringResource>) {
    commands.spawn((
        NodeBundle {
            style: Style {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                padding: UiRect::all(Val::Px(20.0)),
                row_gap: Val::Px(20.0),
                ..default()
            },
            background_color: Color::srgba(0.1, 0.1, 0.1, 0.95).into(),
            ..default()
        },
        LauncherUI,
    )).with_children(|parent| {
        // 标题
        parent.spawn(TextBundle::from_section(
            &ui_strings.strings.launcher.select_world,
            TextStyle {
                font: default(),
                font_size: 28.0,
                color: Color::WHITE,
            },
        ));

        // 世界列表
        parent.spawn(NodeBundle {
            style: Style {
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                row_gap: Val::Px(10.0),
                max_height: Val::Px(300.0),
                overflow: Overflow::clip_y(),
                ..default()
            },
            ..default()
        }).with_children(|parent| {
            for world in &launcher_data.worlds {
                create_world_button(parent, &world.name);
            }
        });

        // 底部按钮
        parent.spawn(NodeBundle {
            style: Style {
                flex_direction: FlexDirection::Row,
                column_gap: Val::Px(20.0),
                ..default()
            },
            ..default()
        }).with_children(|parent| {
            create_launcher_button(parent, &ui_strings.strings.launcher.back, "back");
            create_launcher_button(parent, &ui_strings.strings.launcher.create_world, "create_world");
        });
    });
}

fn setup_settings(mut commands: Commands, ui_strings: Res<UiStringResource>) {
    commands.spawn((
        NodeBundle {
            style: Style {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                row_gap: Val::Px(20.0),
                ..default()
            },
            background_color: Color::srgba(0.1, 0.1, 0.1, 0.95).into(),
            ..default()
        },
        LauncherUI,
    )).with_children(|parent| {
        parent.spawn(TextBundle::from_section(
            &ui_strings.strings.launcher.settings_title,
            TextStyle {
                font: default(),
                font_size: 28.0,
                color: Color::WHITE,
            },
        ));

        parent.spawn(TextBundle::from_section(
            &ui_strings.strings.launcher.settings_placeholder,
            TextStyle {
                font: default(),
                font_size: 16.0,
                color: Color::srgb(0.5, 0.5, 0.5),
            },
        ));

        create_launcher_button(parent, &ui_strings.strings.launcher.back, "back");
    });
}

fn create_launcher_button(parent: &mut ChildBuilder, text: &str, action: &str) {
    parent.spawn((
        ButtonBundle {
            style: Style {
                width: Val::Px(200.0),
                height: Val::Px(45.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                border: UiRect::all(Val::Px(1.0)),
                ..default()
            },
            background_color: Color::srgba(0.2, 0.2, 0.2, 0.9).into(),
            border_color: Color::srgba(0.4, 0.4, 0.4, 0.8).into(),
            ..default()
        },
        Name::new(action.to_string()),
    )).with_children(|parent| {
        parent.spawn(TextBundle::from_section(
            text,
            TextStyle {
                font: default(),
                font_size: 16.0,
                color: Color::WHITE,
            },
        ));
    });
}

fn create_world_button(parent: &mut ChildBuilder, world_name: &str) {
    parent.spawn((
        ButtonBundle {
            style: Style {
                width: Val::Px(400.0),
                height: Val::Px(50.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                border: UiRect::all(Val::Px(1.0)),
                ..default()
            },
            background_color: Color::srgba(0.2, 0.2, 0.2, 0.9).into(),
            border_color: Color::srgba(0.4, 0.4, 0.4, 0.8).into(),
            ..default()
        },
        WorldButton(world_name.to_string()),
    )).with_children(|parent| {
        parent.spawn(TextBundle::from_section(
            world_name,
            TextStyle {
                font: default(),
                font_size: 18.0,
                color: Color::WHITE,
            },
        ));
    });
}

fn main_menu_system(
    mut interaction_query: Query<(&Interaction, &Name), (Changed<Interaction>, With<Button>)>,
    mut next_state: ResMut<NextState<LauncherState>>,
    mut app_exit_events: EventWriter<bevy::app::AppExit>,
) {
    for (interaction, name) in &mut interaction_query {
        if *interaction == Interaction::Pressed {
            match name.as_str() {
                "singleplayer" => {
                    next_state.set(LauncherState::WorldSelection);
                }
                "settings" => {
                    next_state.set(LauncherState::Settings);
                }
                "quit" => {
                    app_exit_events.send(bevy::app::AppExit::Success);
                }
                _ => {}
            }
        }
    }
}

fn world_selection_system(
    mut interaction_query: Query<(&Interaction, Option<&Name>, Option<&WorldButton>), (Changed<Interaction>, With<Button>)>,
    mut next_state: ResMut<NextState<LauncherState>>,
    mut launcher_data: ResMut<LauncherData>,
    ui_strings: Res<UiStringResource>,
) {
    for (interaction, name, world_button) in &mut interaction_query {
        if *interaction == Interaction::Pressed {
            if let Some(name) = name {
                match name.as_str() {
                    "back" => {
                        next_state.set(LauncherState::MainMenu);
                    }
                    "create_world" => {
                        // 这里可以添加创建世界的逻辑
                        println!("{}", ui_strings.strings.launcher.create_world_todo);
                    }
                    _ => {}
                }
            } else if let Some(world_button) = world_button {
                // 启动游戏
                launcher_data.selected_world = Some(world_button.0.clone());
                launch_game(&world_button.0, &ui_strings.strings.launcher);
            }
        }
    }
}

fn settings_system(
    mut interaction_query: Query<(&Interaction, &Name), (Changed<Interaction>, With<Button>)>,
    mut next_state: ResMut<NextState<LauncherState>>,
) {
    for (interaction, name) in &mut interaction_query {
        if *interaction == Interaction::Pressed {
            match name.as_str() {
                "back" => {
                    next_state.set(LauncherState::MainMenu);
                }
                _ => {}
            }
        }
    }
}

fn cleanup_ui(mut commands: Commands, query: Query<Entity, With<LauncherUI>>) {
    for entity in &query {
        commands.entity(entity).despawn_recursive();
    }
}

fn load_ui_strings() -> UiStrings {
    // 尝试从配置文件加载UI字符串
    let config_path = "../ui_strings.json";
    
    if let Ok(content) = std::fs::read_to_string(config_path) {
        if let Ok(ui_strings) = serde_json::from_str::<UiStrings>(&content) {
            return ui_strings;
        }
    }
    
    // 如果加载失败，返回默认的英文字符串
    UiStrings {
        launcher: LauncherStrings {
            title: "Minecraft Rust Launcher".to_string(),
            singleplayer: "Singleplayer".to_string(),
            settings: "Settings".to_string(),
            quit: "Quit".to_string(),
            select_world: "Select World".to_string(),
            back: "Back".to_string(),
            create_world: "Create New World".to_string(),
            settings_title: "Settings".to_string(),
            settings_placeholder: "Launcher settings will be displayed here".to_string(),
            world_examples: WorldExamples {
                my_world: "My World".to_string(),
                survival_world: "Survival World".to_string(),
            },
            launch_game: "Launching game, world: ".to_string(),
            game_started: "Game started, PID: ".to_string(),
            launch_failed: "Failed to launch game: ".to_string(),
            create_world_todo: "Create new world feature to be implemented".to_string(),
        },
    }
}

fn load_worlds() -> Vec<WorldInfo> {
    // 这里应该从文件系统加载世界列表
    // 现在返回一些示例数据，使用英文名称
    vec![
        WorldInfo {
            name: "My World".to_string(),
            game_mode: "creative".to_string(),
            world_type: "default".to_string(),
            last_played: "2024-01-15".to_string(),
        },
        WorldInfo {
            name: "Survival World".to_string(),
            game_mode: "survival".to_string(),
            world_type: "default".to_string(),
            last_played: "2024-01-14".to_string(),
        },
    ]
}

fn launch_game(world_name: &str, strings: &LauncherStrings) {
    println!("{}{}", strings.launch_game, world_name);
    
    let game_path = if cfg!(target_os = "windows") {
        // 优先尝试 release 版本
        if std::path::Path::new("../target/release/minecraft_rust.exe").exists() {
            "../target/release/minecraft_rust.exe"
        } else {
            "../target/debug/minecraft_rust.exe"
        }
    } else {
        // 优先尝试 release 版本
        if std::path::Path::new("../target/release/minecraft_rust").exists() {
            "../target/release/minecraft_rust"
        } else {
            "../target/debug/minecraft_rust"
        }
    };
    
    match std::process::Command::new(game_path)
        .arg("--world")
        .arg(world_name)
        .spawn()
    {
        Ok(child) => {
            println!("{}{}", strings.game_started, child.id());
        }
        Err(e) => {
            eprintln!("{}{}", strings.launch_failed, e);
        }
    }
}