use bevy::prelude::*;
use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use crate::scripting::ScriptEngine;
use crate::world::chunk::BlockId;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptBlockDefinition {
    pub id: String,
    pub hardness: f32,
    pub transparent: bool,
    pub solid: bool,
    pub texture: Option<String>,
    pub light_level: u8,
}

impl Default for ScriptBlockDefinition {
    fn default() -> Self {
        Self {
            id: "unknown".to_string(),
            hardness: 1.0,
            transparent: false,
            solid: true,
            texture: None,
            light_level: 0,
        }
    }
}

#[derive(Resource, Default, Clone)]
pub struct BlockRegistry {
    pub definitions: HashMap<String, ScriptBlockDefinition>,
    pub id_to_blockid: HashMap<String, BlockId>,
}

impl BlockRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn load_from_scripts(&mut self, script_engine: &ScriptEngine) -> Result<(), mlua::Error> {
        info!("Loading block definitions from separate Lua script files...");
        
        // 需要加载的方块类型
        let block_names = vec!["stone", "dirt", "grass", "bedrock"];
        
        for block_name in block_names {
            let script_path = format!("{}.lua", block_name);
            
            // 尝试加载该方块的脚本
            match script_engine.load_file(&script_path) {
                Ok(_) => {
                    // 执行脚本，获取返回的定义表
                    script_engine.with_lua(|lua| {
                        // 读取文件并执行
                        let script_content = std::fs::read_to_string(script_engine.root().join(&script_path))
                            .map_err(|e| mlua::Error::external(format!("Failed to read {}: {}", script_path, e)))?;
                        
                        // 执行并获取返回值
                        let block_def = lua.load(&script_content)
                            .set_name(&script_path)
                            .eval::<mlua::Table>()?;
                        
                        let mut definition = ScriptBlockDefinition::default();
                        definition.id = block_name.to_string();
                        
                        // 读取方块属性
                        if let Ok(hardness) = block_def.get::<_, f32>("hardness") {
                            definition.hardness = hardness;
                        }
                        
                        if let Ok(transparent) = block_def.get::<_, bool>("transparent") {
                            definition.transparent = transparent;
                        }
                        
                        if let Ok(solid) = block_def.get::<_, bool>("solid") {
                            definition.solid = solid;
                        }
                        
                        if let Ok(texture) = block_def.get::<_, String>("texture") {
                            definition.texture = Some(texture);
                        }
                        
                        if let Ok(light_level) = block_def.get::<_, u8>("light_level") {
                            definition.light_level = light_level;
                        }
                        
                        info!("Registered script block: {} (hardness: {}, texture: {:?})", 
                              definition.id, definition.hardness, definition.texture);
                        
                        // 映射到对应的 BlockId
                        let block_id = match definition.id.as_str() {
                            "stone" => BlockId::Stone,
                            "dirt" => BlockId::Dirt,
                            "grass" => BlockId::Grass,
                            "bedrock" => BlockId::Bedrock,
                            _ => BlockId::Stone, // 默认映射
                        };
                        
                        self.id_to_blockid.insert(definition.id.clone(), block_id);
                        self.definitions.insert(definition.id.clone(), definition);
                        
                        Ok(())
                    })?;
                }
                Err(e) => {
                    warn!("Failed to load block script '{}': {}", script_path, e);
                }
            }
        }
        
        info!("Loaded {} block definitions from separate script files", self.definitions.len());
        Ok(())
    }

    pub fn get_definition(&self, id: &str) -> Option<&ScriptBlockDefinition> {
        self.definitions.get(id)
    }

    pub fn get_block_id(&self, script_id: &str) -> Option<BlockId> {
        self.id_to_blockid.get(script_id).copied()
    }

    pub fn call_block_event(&self, script_engine: &ScriptEngine, block_id: &str, event: &str, args: String) -> Result<String, mlua::Error> {
        script_engine.with_lua(|lua| {
            let globals = lua.globals();
            
            if let Ok(blocks_table) = globals.get::<_, mlua::Table>("blocks") {
                if let Ok(block_def) = blocks_table.get::<_, mlua::Table>(block_id) {
                    if let Ok(event_func) = block_def.get::<_, mlua::Function>(event) {
                        let result = event_func.call::<_, mlua::Value>(args)?;
                        match result {
                            mlua::Value::String(s) => return Ok(s.to_str()?.to_string()),
                            mlua::Value::Number(n) => return Ok(n.to_string()),
                            mlua::Value::Boolean(b) => return Ok(b.to_string()),
                            _ => return Ok("nil".to_string()),
                        }
                    }
                }
            }
            
            Ok("no_event".to_string())
        })
    }

    pub fn get_all_registered_blocks(&self) -> Vec<&ScriptBlockDefinition> {
        self.definitions.values().collect()
    }
}