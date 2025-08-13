use bevy::prelude::*;
use mlua::{Function, Result as LuaResult};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

#[derive(Resource, Clone)]
pub struct ScriptEngine {
    lua: Arc<Mutex<mlua::Lua>>, // guard Lua to satisfy Sync for Bevy resources
    root: PathBuf,
}

impl Default for ScriptEngine {
    fn default() -> Self {
        Self { lua: Arc::new(Mutex::new(mlua::Lua::new())), root: PathBuf::from("scripts") }
    }
}

impl ScriptEngine {
    pub fn new<P: Into<PathBuf>>(root: P) -> Self {
        Self { lua: Arc::new(Mutex::new(mlua::Lua::new())), root: root.into() }
    }

    pub fn root(&self) -> &Path { &self.root }

    pub fn set_root<P: Into<PathBuf>>(&mut self, root: P) { self.root = root.into(); }

    pub fn load_all(&self) -> LuaResult<()> {
        self.ensure_root_dir();
        self.load_dir_recursively(&self.root)
    }

    pub fn load_file<P: AsRef<Path>>(&self, path: P) -> LuaResult<()> {
        let p = path.as_ref();
        let full = if p.is_absolute() { p.to_path_buf() } else { self.root.join(p) };
        let code = fs::read_to_string(&full)
            .map_err(|e| mlua::Error::external(format!("Failed to read {:?}: {}", full, e)))?;
        let lua = self.lua.lock().expect("Lua poisoned");
        lua.load(&code).set_name(full.to_string_lossy().to_string()).exec()?;
        Ok(())
    }

    pub fn call0<T: for<'lua> mlua::FromLuaMulti<'lua>>(&self, name: &str) -> LuaResult<T> {
        let lua = self.lua.lock().expect("Lua poisoned");
        let globals = lua.globals();
        let func: Function = globals.get(name)?;
        func.call(())
    }

    pub fn call1<A: for<'lua> mlua::IntoLuaMulti<'lua>, T: for<'lua> mlua::FromLuaMulti<'lua>>(&self, name: &str, arg: A) -> LuaResult<T> {
        let lua = self.lua.lock().expect("Lua poisoned");
        let globals = lua.globals();
        let func: Function = globals.get(name)?;
        func.call(arg)
    }

    // Provide an HRTB helper to work with Lua values safely within its lifetime
    pub fn with_lua<R, F>(&self, f: F) -> LuaResult<R>
    where
        F: for<'lua> FnOnce(&'lua mlua::Lua) -> LuaResult<R>,
    {
        let lua = self.lua.lock().expect("Lua poisoned");
        f(&lua)
    }

    fn ensure_root_dir(&self) {
        if !self.root.exists() {
            let _ = fs::create_dir_all(&self.root);
        }
    }

    fn load_dir_recursively(&self, dir: &Path) -> LuaResult<()> {
        if !dir.exists() { return Ok(()); }
        for entry in fs::read_dir(dir).map_err(|e| mlua::Error::external(format!("read_dir {:?} failed: {}", dir, e)))? {
            let entry = entry.map_err(|e| mlua::Error::external(format!("read_dir entry error: {}", e)))?;
            let path = entry.path();
            if path.is_dir() {
                self.load_dir_recursively(&path)?;
            } else if path.extension().map(|e| e == "lua").unwrap_or(false) {
                let code = fs::read_to_string(&path)
                    .map_err(|e| mlua::Error::external(format!("Failed to read {:?}: {}", path, e)))?;
                let lua = self.lua.lock().expect("Lua poisoned");
                lua.load(&code).set_name(path.to_string_lossy().to_string()).exec()?;
            }
        }
        Ok(())
    }
}