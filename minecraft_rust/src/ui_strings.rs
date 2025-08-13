use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// UI字符串配置
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct UiStrings {
    pub pause_menu: PauseMenuStrings,
    pub hud: HudStrings,
    pub launcher: LauncherStrings,
    pub errors: ErrorStrings,
    pub game: GameStrings,
    pub common: CommonStrings,
}



#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PauseMenuStrings {
    pub title: String,
    pub continue_game: String,
    pub quit: String,
    pub hint: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct HudStrings {
    pub items: HashMap<String, String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
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
    pub world_examples: HashMap<String, String>,
    pub launch_game: String,
    pub game_started: String,
    pub launch_failed: String,
    pub create_world_todo: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ErrorStrings {
    pub world_exists: String,
    pub script_load_failed: String,
    pub block_load_failed: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GameStrings {
    pub controls_hint: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CommonStrings {
    pub off: String,
    pub on: String,
    pub low: String,
    pub medium: String,
    pub high: String,
    pub ultra: String,
    pub none: String,
    pub back: String,
    pub save: String,
}

/// UI字符串管理器资源
#[derive(Resource, Debug, Clone)]
pub struct UiStringManager {
    pub strings: UiStrings,
}

impl Default for UiStringManager {
    fn default() -> Self {
        Self::new()
    }
}

impl UiStringManager {
    pub fn new() -> Self {
        let strings = Self::load_strings().unwrap_or_else(|e| {
            warn!("Failed to load UI strings: {}, using defaults", e);
            Self::default_strings()
        });
        
        Self { strings }
    }
    
    fn load_strings() -> Result<UiStrings, Box<dyn std::error::Error>> {
        let content = std::fs::read_to_string("ui_strings.json")?;
        let strings: UiStrings = serde_json::from_str(&content)?;
        Ok(strings)
    }
    
    fn default_strings() -> UiStrings {
        UiStrings {
            pause_menu: PauseMenuStrings {
                title: "Game Paused".to_string(),
                continue_game: "Continue Game".to_string(),
                quit: "Quit Game".to_string(),
                hint: "Press ESC to continue".to_string(),
            },
            hud: HudStrings {
                items: {
                    let mut items = HashMap::new();
                    items.insert("grass_block".to_string(), "Grass Block".to_string());
                    items.insert("dirt".to_string(), "Dirt".to_string());
                    items.insert("stone".to_string(), "Stone".to_string());
                    items.insert("bedrock".to_string(), "Bedrock".to_string());
                    items.insert("air".to_string(), "Air".to_string());
                    items.insert("wooden_pickaxe".to_string(), "Wooden Pickaxe".to_string());
                    items.insert("stone_pickaxe".to_string(), "Stone Pickaxe".to_string());
                    items.insert("iron_pickaxe".to_string(), "Iron Pickaxe".to_string());
                    items.insert("diamond_pickaxe".to_string(), "Diamond Pickaxe".to_string());
                    items
                },
            },
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
                world_examples: {
                    let mut examples = HashMap::new();
                    examples.insert("my_world".to_string(), "My World".to_string());
                    examples.insert("survival_world".to_string(), "Survival World".to_string());
                    examples
                },
                launch_game: "Launching game, world: ".to_string(),
                game_started: "Game started, PID: ".to_string(),
                launch_failed: "Failed to launch game: ".to_string(),
                create_world_todo: "Create new world feature to be implemented".to_string(),
            },
            errors: ErrorStrings {
                world_exists: "World already exists".to_string(),
                script_load_failed: "Failed to load Lua scripts: ".to_string(),
                block_load_failed: "Failed to load blocks from scripts: ".to_string(),
            },
            game: GameStrings {
                controls_hint: "Use WASD to move, mouse to look around, ESC to pause".to_string(),
            },
            common: CommonStrings {
                off: "Off".to_string(),
                on: "On".to_string(),
                low: "Low".to_string(),
                medium: "Medium".to_string(),
                high: "High".to_string(),
                ultra: "Ultra".to_string(),
                none: "None".to_string(),
                back: "Back".to_string(),
                save: "Save".to_string(),
            },
        }
    }
    
    /// 获取物品显示名称
    pub fn get_item_name<'a>(&'a self, item_key: &'a str) -> &'a str {
        self.strings.hud.items.get(item_key).map(|s| s.as_str()).unwrap_or(item_key)
    }
}