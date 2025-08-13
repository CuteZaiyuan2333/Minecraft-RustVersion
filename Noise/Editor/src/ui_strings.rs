use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Clone)]
#[serde(default)]
pub struct UiStrings {
    pub app: AppStrings,
    pub menu: MenuStrings,
    pub graph_panel: GraphPanelStrings,
    pub preview: PreviewStrings,
}

#[derive(Debug, Deserialize, Clone, Default)]
#[serde(default)]
pub struct AppStrings {
    pub window_title: String,
}

#[derive(Debug, Deserialize, Clone, Default)]
#[serde(default)]
pub struct MenuStrings {
    pub file: String,
    pub bake: String,
    pub save: String,
    pub load: String,
}

#[derive(Debug, Deserialize, Clone, Default)]
#[serde(default)]
pub struct GraphPanelStrings {
    pub title: String,
    pub hint: String,
    pub add_node: String,
    pub clear: String,
    pub node_prefix: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(default)]
pub struct PreviewStrings {
    pub title: String,
    pub resolution: String,
    pub generate: String,
    pub channel: String,
    pub r: String,
    pub g: String,
    pub b: String,
    pub width_short: String,
    pub height_short: String,
    pub open_window: String,
    pub window_title: String,
}

impl Default for UiStrings {
    fn default() -> Self {
        Self {
            app: AppStrings {
                window_title: "Noise Editor".to_string(),
            },
            menu: MenuStrings {
                file: "File".to_string(),
                save: "Save".to_string(),
                load: "Load".to_string(),
                bake: "Bake".to_string(),
            },
            graph_panel: GraphPanelStrings {
                title: "Graph".to_string(),
                hint: "Edit nodes and connections".to_string(),
                add_node: "Add Node".to_string(),
                clear: "Clear".to_string(),
                node_prefix: "Node".to_string(),
            },
            preview: PreviewStrings {
                title: "Preview".to_string(),
                resolution: "Resolution".to_string(),
                generate: "Generate".to_string(),
                channel: "Channel".to_string(),
                r: "R".to_string(),
                g: "G".to_string(),
                b: "B".to_string(),
                width_short: "W".to_string(),
                height_short: "H".to_string(),
                open_window: "Open Preview Window".to_string(),
                window_title: "Preview".to_string(),
            },
        }
    }
}

pub fn load_from_file(path: &str) -> anyhow::Result<UiStrings> {
    let s = std::fs::read_to_string(path)?;
    let ui: UiStrings = serde_json::from_str(&s)?;
    Ok(ui)
}