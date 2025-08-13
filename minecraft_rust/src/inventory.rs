use bevy::prelude::*;
use crate::world::chunk::BlockId;
use crate::game_state::GameState;

/// 物品栏槽位
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ItemStack {
    pub item_type: ItemType,
    pub count: u32,
}

/// 物品类型
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ItemType {
    Block(BlockId),
    Tool(ToolType),
    Empty,
}

/// 工具类型
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ToolType {
    WoodenPickaxe,
    StonePickaxe,
    IronPickaxe,
    DiamondPickaxe,
}

impl ItemStack {
    pub fn new(item_type: ItemType, count: u32) -> Self {
        Self { item_type, count }
    }

    pub fn empty() -> Self {
        Self {
            item_type: ItemType::Empty,
            count: 0,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.item_type == ItemType::Empty || self.count == 0
    }

    pub fn can_stack_with(&self, other: &ItemStack) -> bool {
        self.item_type == other.item_type && !self.is_empty() && !other.is_empty()
    }

    pub fn max_stack_size(&self) -> u32 {
        match self.item_type {
            ItemType::Block(_) => 64,
            ItemType::Tool(_) => 1,
            ItemType::Empty => 0,
        }
    }
}

/// 玩家物品栏组件
#[derive(Component)]
pub struct PlayerInventory {
    pub hotbar: [ItemStack; 9],     // 快捷栏
    pub main: [ItemStack; 27],      // 主物品栏
    pub selected_slot: usize,       // 当前选中的快捷栏槽位
}

impl Default for PlayerInventory {
    fn default() -> Self {
        Self {
            hotbar: [ItemStack::empty(); 9],
            main: [ItemStack::empty(); 27],
            selected_slot: 0,
        }
    }
}

impl PlayerInventory {
    pub fn new() -> Self {
        let mut inventory = Self::default();
        
        // 给玩家一些初始物品
        inventory.hotbar[0] = ItemStack::new(ItemType::Block(BlockId::Grass), 64);
        inventory.hotbar[1] = ItemStack::new(ItemType::Block(BlockId::Dirt), 64);
        inventory.hotbar[2] = ItemStack::new(ItemType::Block(BlockId::Stone), 64);
        inventory.hotbar[3] = ItemStack::new(ItemType::Block(BlockId::Bedrock), 64);
        inventory.hotbar[4] = ItemStack::new(ItemType::Tool(ToolType::DiamondPickaxe), 1);
        
        inventory
    }

    pub fn get_selected_item(&self) -> &ItemStack {
        &self.hotbar[self.selected_slot]
    }

    pub fn get_selected_item_mut(&mut self) -> &mut ItemStack {
        &mut self.hotbar[self.selected_slot]
    }

    pub fn select_slot(&mut self, slot: usize) {
        if slot < 9 {
            self.selected_slot = slot;
        }
    }

    pub fn add_item(&mut self, item: ItemStack) -> ItemStack {
        if item.is_empty() {
            return item;
        }

        let mut remaining = item;

        // 首先尝试堆叠到现有物品
        for slot in self.hotbar.iter_mut().chain(self.main.iter_mut()) {
            if slot.can_stack_with(&remaining) {
                let max_add = slot.max_stack_size() - slot.count;
                let add_count = remaining.count.min(max_add);
                
                slot.count += add_count;
                remaining.count -= add_count;
                
                if remaining.count == 0 {
                    return ItemStack::empty();
                }
            }
        }

        // 然后尝试放入空槽位
        for slot in self.hotbar.iter_mut().chain(self.main.iter_mut()) {
            if slot.is_empty() {
                *slot = remaining;
                return ItemStack::empty();
            }
        }

        // 物品栏已满，返回剩余物品
        remaining
    }

    pub fn remove_item(&mut self, item_type: ItemType, count: u32) -> u32 {
        let mut removed = 0;
        let mut remaining = count;

        for slot in self.hotbar.iter_mut().chain(self.main.iter_mut()) {
            if slot.item_type == item_type && !slot.is_empty() {
                let remove_count = remaining.min(slot.count);
                slot.count -= remove_count;
                removed += remove_count;
                remaining -= remove_count;

                if slot.count == 0 {
                    *slot = ItemStack::empty();
                }

                if remaining == 0 {
                    break;
                }
            }
        }

        removed
    }
}

/// 物品栏系统
pub fn inventory_input_system(
    keyboard: Res<Input<KeyCode>>,
    mut inventory_query: Query<&mut PlayerInventory>,
) {
    for mut inventory in inventory_query.iter_mut() {
        // 数字键选择快捷栏槽位
        for i in 0..9 {
            let key = match i {
                0 => KeyCode::Key1,
                1 => KeyCode::Key2,
                2 => KeyCode::Key3,
                3 => KeyCode::Key4,
                4 => KeyCode::Key5,
                5 => KeyCode::Key6,
                6 => KeyCode::Key7,
                7 => KeyCode::Key8,
                8 => KeyCode::Key9,
                _ => continue,
            };

            if keyboard.just_pressed(key) {
                inventory.select_slot(i);
                println!("选择快捷栏槽位: {}", i + 1);
            }
        }
    }
}

/// 物品栏插件
pub struct InventoryPlugin;

impl Plugin for InventoryPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, inventory_input_system.run_if(in_state(GameState::InGame)));
    }
}