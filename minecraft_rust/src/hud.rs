use bevy::prelude::*;
use crate::inventory::{PlayerInventory, ItemType};
use crate::world::chunk::BlockId;
use crate::game_state::GameState;
use crate::ui_strings::UiStringManager;

/// HUD根节点标记
#[derive(Component)]
pub struct HudRoot;

/// 快捷栏UI标记
#[derive(Component)]
pub struct HotbarUI;

/// 快捷栏槽位UI标记
#[derive(Component)]
pub struct HotbarSlot {
    pub slot_index: usize,
}

/// 物品数量文本标记
#[derive(Component)]
pub struct ItemCountText {
    pub slot_index: usize,
}

/// HUD插件
pub struct HudPlugin;

impl Plugin for HudPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::InGame), setup_hud)
           .add_systems(Update, (update_hotbar_ui, update_item_count_text).run_if(in_state(GameState::InGame)));
    }
}

fn setup_hud(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    ui_strings: Res<UiStringManager>,
) {
    // 创建HUD根节点
    let hud_root = commands.spawn((
        NodeBundle {
            style: Style {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                position_type: PositionType::Absolute,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::FlexEnd,
                ..default()
            },
            ..default()
        },
        HudRoot,
    )).id();

    // 创建快捷栏容器
    let hotbar_container = commands.spawn((
        NodeBundle {
            style: Style {
                width: Val::Px(360.0), // 9个槽位 * 40px
                height: Val::Px(40.0),
                margin: UiRect::bottom(Val::Px(20.0)),
                justify_content: JustifyContent::SpaceBetween,
                align_items: AlignItems::Center,
                ..default()
            },
            background_color: Color::rgba(0.0, 0.0, 0.0, 0.5).into(),
            ..default()
        },
        HotbarUI,
    )).id();

    commands.entity(hud_root).push_children(&[hotbar_container]);

    // 创建9个快捷栏槽位
    for i in 0..9 {
        let slot = commands.spawn((
            NodeBundle {
                style: Style {
                    width: Val::Px(36.0),
                    height: Val::Px(36.0),
                    margin: UiRect::all(Val::Px(2.0)),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                background_color: if i == 0 { 
                    Color::rgba(1.0, 1.0, 1.0, 0.3) // 选中状态
                } else { 
                    Color::rgba(0.5, 0.5, 0.5, 0.3) // 未选中状态
                }.into(),
                ..default()
            },
            HotbarSlot { slot_index: i },
        )).id();

        // 添加物品数量文本
        let count_text = commands.spawn((
            TextBundle::from_section(
                "",
                TextStyle {
                    font: default(),
                    font_size: 12.0,
                    color: Color::WHITE,
                },
            ).with_style(Style {
                position_type: PositionType::Absolute,
                bottom: Val::Px(2.0),
                right: Val::Px(2.0),
                ..default()
            }),
            ItemCountText { slot_index: i },
        )).id();

        commands.entity(hotbar_container).push_children(&[slot]);
        commands.entity(slot).push_children(&[count_text]);
    }
}

fn update_hotbar_ui(
    inventory_query: Query<&PlayerInventory>,
    mut slot_query: Query<(&HotbarSlot, &mut BackgroundColor)>,
) {
    if let Ok(inventory) = inventory_query.get_single() {
        for (slot, mut bg_color) in slot_query.iter_mut() {
            if slot.slot_index == inventory.selected_slot {
                *bg_color = Color::rgba(1.0, 1.0, 1.0, 0.5).into(); // 选中状态
            } else {
                *bg_color = Color::rgba(0.5, 0.5, 0.5, 0.3).into(); // 未选中状态
            }
        }
    }
}

fn update_item_count_text(
    inventory_query: Query<&PlayerInventory>,
    mut text_query: Query<(&ItemCountText, &mut Text)>,
    ui_strings: Res<UiStringManager>,
) {
    if let Ok(inventory) = inventory_query.get_single() {
        for (count_text, mut text) in text_query.iter_mut() {
            let item = &inventory.hotbar[count_text.slot_index];
            
            if item.is_empty() {
                text.sections[0].value = "".to_string();
            } else {
                // 显示物品类型和数量
                let item_key = match item.item_type {
                    ItemType::Block(BlockId::Grass) => "grass_block",
                    ItemType::Block(BlockId::Dirt) => "dirt",
                    ItemType::Block(BlockId::Stone) => "stone",
                    ItemType::Block(BlockId::Bedrock) => "bedrock",
                    ItemType::Block(BlockId::Air) => "air",
                    ItemType::Tool(tool_type) => match tool_type {
                        crate::inventory::ToolType::WoodenPickaxe => "wooden_pickaxe",
                        crate::inventory::ToolType::StonePickaxe => "stone_pickaxe",
                        crate::inventory::ToolType::IronPickaxe => "iron_pickaxe",
                        crate::inventory::ToolType::DiamondPickaxe => "diamond_pickaxe",
                    },
                    ItemType::Empty => "",
                };
                let item_name = ui_strings.get_item_name(item_key);
                
                if item.count > 1 {
                    text.sections[0].value = format!("{}\n{}", item_name, item.count);
                } else {
                    text.sections[0].value = item_name.to_string();
                }
            }
        }
    }
}