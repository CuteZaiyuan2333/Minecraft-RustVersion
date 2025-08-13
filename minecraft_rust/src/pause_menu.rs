use bevy::prelude::*;
use crate::game_state::{GameState, WorldManager};
use crate::ui_strings::UiStringManager;

/// 暂停菜单UI标记
#[derive(Component)]
pub struct PauseMenuUI;

/// 暂停菜单插件
pub struct PauseMenuPlugin;

impl Plugin for PauseMenuPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::Paused), setup_pause_menu)
           .add_systems(OnExit(GameState::Paused), cleanup_pause_menu)
           .add_systems(Update, pause_menu_button_system.run_if(in_state(GameState::Paused)));
    }
}

/// 设置暂停菜单
fn setup_pause_menu(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    ui_strings: Res<UiStringManager>,
) {
    // 暂停菜单容器
    commands.spawn((
        NodeBundle {
            style: Style {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            background_color: Color::rgba(0.0, 0.0, 0.0, 0.7).into(),
            ..default()
        },
        PauseMenuUI,
    )).with_children(|parent| {
        // 暂停标题
        parent.spawn(TextBundle::from_section(
            &ui_strings.strings.pause_menu.title,
            TextStyle {
                font: default(),
                font_size: 48.0,
                color: Color::WHITE,
            },
        ).with_style(Style {
            margin: UiRect::bottom(Val::Px(40.0)),
            ..default()
        }));

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
            // 继续游戏按钮
            create_pause_button(parent, &asset_server, &ui_strings.strings.pause_menu.continue_game, "resume");
            
            // 退出游戏按钮
            create_pause_button(parent, &asset_server, &ui_strings.strings.pause_menu.quit, "quit_game");
        });

        // 提示文本
        parent.spawn(TextBundle::from_section(
            &ui_strings.strings.game.controls_hint,
            TextStyle {
                font: default(),
                font_size: 16.0,
                color: Color::GRAY,
            },
        ).with_style(Style {
            margin: UiRect::top(Val::Px(30.0)),
            ..default()
        }));
    });
}

/// 创建暂停菜单按钮
fn create_pause_button(
    parent: &mut ChildBuilder,
    asset_server: &AssetServer,
    text: &str,
    action: &str,
) {
    parent.spawn((
        ButtonBundle {
            style: Style {
                width: Val::Px(250.0),
                height: Val::Px(50.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            background_color: Color::rgba(0.3, 0.3, 0.3, 0.9).into(),
            ..default()
        },
        Name::new(action.to_string()),
    )).with_children(|parent| {
        parent.spawn(TextBundle::from_section(
            text,
            TextStyle {
                font: default(),
                font_size: 20.0,
                color: Color::WHITE,
            },
        ));
    });
}

/// 暂停菜单按钮系统
fn pause_menu_button_system(
    mut interaction_query: Query<(&Interaction, &Name), (Changed<Interaction>, With<Button>)>,
    mut next_state: ResMut<NextState<GameState>>,
    mut world_manager: ResMut<WorldManager>,
    mut windows: Query<&mut Window>,
    mut app_exit_events: EventWriter<bevy::app::AppExit>,
    mut commands: Commands,
    mut save_queue: ResMut<crate::game_state::SaveQueue>,
) {
    for (interaction, name) in &mut interaction_query {
        if *interaction == Interaction::Pressed {
            match name.as_str() {
                "resume" => {
                    next_state.set(GameState::InGame);
                    // 重新锁定鼠标
                    if let Ok(mut window) = windows.get_single_mut() {
                        window.cursor.grab_mode = bevy::window::CursorGrabMode::Confined;
                        window.cursor.visible = false;
                    }
                }

                "quit_game" => {
                    // 保存当前世界（如果有的话）
                    if let Some(current_world) = world_manager.current_world.clone() {
                        world_manager.update_last_played(&current_world);
                        // 异步保存世界信息
                        world_manager.save_world_info_async(&current_world, &mut commands, &mut save_queue);
                        info!("Saved world before quitting: {}", current_world);
                    }
                    
                    // 退出游戏
                    app_exit_events.send(bevy::app::AppExit);
                }
                _ => {}
            }
        }
    }
}

/// 清理暂停菜单
fn cleanup_pause_menu(mut commands: Commands, query: Query<Entity, With<PauseMenuUI>>) {
    for entity in &query {
        commands.entity(entity).despawn_recursive();
    }
}