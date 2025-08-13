use std::collections::HashMap;
use std::fs;
use std::path::Path;
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LanguageInfo {
    pub name: String,
    pub code: String,
    pub author: String,
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LanguageData {
    pub language_info: LanguageInfo,
    #[serde(flatten)]
    pub texts: serde_json::Value,
}

#[derive(Debug, Clone, Resource)]
pub struct LocalizationManager {
    current_language: String,
    available_languages: Vec<LanguageInfo>,
    texts: HashMap<String, String>,
}

impl Default for LocalizationManager {
    fn default() -> Self {
        let mut manager = Self {
            current_language: "en_us".to_string(),
            available_languages: Vec::new(),
            texts: HashMap::new(),
        };
        
        // Load available languages
        manager.scan_languages();
        
        // Load default language (English)
        if let Err(e) = manager.load_language("en_us") {
            warn!("Failed to load default language: {}", e);
            manager.load_fallback_texts();
        }
        
        manager
    }
}

impl LocalizationManager {
    pub fn new() -> Self {
        Self::default()
    }
    
    pub fn scan_languages(&mut self) {
        self.available_languages.clear();
        
        let localization_dir = "localization";
        if let Ok(entries) = fs::read_dir(localization_dir) {
            for entry in entries.flatten() {
                if let Some(file_name) = entry.file_name().to_str() {
                    if file_name.ends_with(".json") {
                        let lang_code = file_name.trim_end_matches(".json");
                        if let Ok(lang_data) = self.load_language_info(&entry.path()) {
                            self.available_languages.push(lang_data.language_info);
                        } else {
                            // Fallback for files without proper language_info
                            self.available_languages.push(LanguageInfo {
                                name: lang_code.to_string(),
                                code: lang_code.to_string(),
                                author: "Unknown".to_string(),
                                version: "1.0.0".to_string(),
                            });
                        }
                    }
                }
            }
        }
        
        info!("Found {} languages", self.available_languages.len());
    }
    
    fn load_language_info(&self, path: &Path) -> Result<LanguageData, Box<dyn std::error::Error>> {
        let content = fs::read_to_string(path)?;
        let data: LanguageData = serde_json::from_str(&content)?;
        Ok(data)
    }
    
    pub fn load_language(&mut self, language_code: &str) -> Result<(), Box<dyn std::error::Error>> {
        let file_path = format!("localization/{}.json", language_code);
        
        if !Path::new(&file_path).exists() {
            return Err(format!("Language file not found: {}", file_path).into());
        }
        
        let content = fs::read_to_string(&file_path)?;
        let data: LanguageData = serde_json::from_str(&content)?;
        
        self.texts.clear();
        self.flatten_json(&data.texts, String::new());
        self.current_language = language_code.to_string();
        
        info!("Loaded language: {} ({})", data.language_info.name, language_code);
        Ok(())
    }
    
    fn flatten_json(&mut self, value: &serde_json::Value, prefix: String) {
        match value {
            serde_json::Value::Object(map) => {
                for (key, val) in map {
                    let new_key = if prefix.is_empty() {
                        key.clone()
                    } else {
                        format!("{}.{}", prefix, key)
                    };
                    self.flatten_json(val, new_key);
                }
            }
            serde_json::Value::String(s) => {
                self.texts.insert(prefix, s.clone());
            }
            _ => {}
        }
    }
    
    fn load_fallback_texts(&mut self) {
        // Fallback English texts if no language files are available
        self.texts.insert("ui.main_menu.title".to_string(), "Minecraft Rust".to_string());
        self.texts.insert("ui.main_menu.singleplayer".to_string(), "Singleplayer".to_string());
        self.texts.insert("ui.main_menu.settings".to_string(), "Settings".to_string());
        self.texts.insert("ui.main_menu.quit".to_string(), "Quit Game".to_string());
        self.texts.insert("game.info.fps".to_string(), "FPS".to_string());
        self.texts.insert("game.info.chunks_loaded".to_string(), "Chunks Loaded".to_string());
        self.texts.insert("game.controls.hint".to_string(), "WASD to move, Mouse to look, Esc to pause".to_string());
        self.texts.insert("graphics.msaa".to_string(), "Anti-Aliasing (MSAA)".to_string());
        self.texts.insert("values.off".to_string(), "Off".to_string());
        self.texts.insert("values.low".to_string(), "Low".to_string());
        self.texts.insert("values.medium".to_string(), "Medium".to_string());
        self.texts.insert("values.high".to_string(), "High".to_string());
        self.texts.insert("values.ultra".to_string(), "Ultra".to_string());
        self.texts.insert("common.close".to_string(), "Close".to_string());
        self.texts.insert("common.restore_defaults".to_string(), "Restore Defaults".to_string());
    }
    
    pub fn get<'a>(&'a self, key: &'a str) -> &'a str {
        self.texts.get(key).map(|s| s.as_str()).unwrap_or(key)
    }
    
    pub fn get_formatted(&self, key: &str, value: f32) -> String {
        match key {
            "render_distance_value" => format!("{:.0}m", value),
            "resolution_scaling_value" => format!("{:.1}x", value),
            _ => self.get(key).to_string(),
        }
    }
    
    pub fn get_current_language(&self) -> &str {
        &self.current_language
    }
    
    pub fn get_available_languages(&self) -> &[LanguageInfo] {
        &self.available_languages
    }
    
    pub fn set_language(&mut self, language_code: &str) -> Result<(), Box<dyn std::error::Error>> {
        self.load_language(language_code)
    }
}

// Event for language change
#[derive(Event)]
pub struct LanguageChangeEvent {
    pub new_language: String,
}

// System to handle language changes
pub fn handle_language_change(
    mut events: EventReader<LanguageChangeEvent>,
    mut localization: ResMut<LocalizationManager>,
) {
    for event in events.read() {
        if let Err(e) = localization.set_language(&event.new_language) {
            error!("Failed to change language to {}: {}", event.new_language, e);
        }
    }
}