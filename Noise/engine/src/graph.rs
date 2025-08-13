use serde::{Deserialize, Serialize}; use petgraph::graph::NodeIndex; use petgraph::stable_graph::StableDiGraph;
#[derive(Debug, Clone, Serialize, Deserialize)] pub enum NodeKind { Constant(f32), Add, Sub, Mul, Div, Min, Max, Abs, Clamp { min: f32, max: f32 }, FnlSimplex2D { freq: f32 }, FnlPerlin2D { freq: f32 }, FnlSimplex3D { freq: f32 }, FnlPerlin3D { freq: f32 }, Translate { dx: f32, dy: f32, dz: f32 }, Scale { sx: f32, sy: f32, sz: f32 } }
#[derive(Debug, Clone, Serialize, Deserialize)] pub struct Node { pub id: u64, pub name: String, pub kind: NodeKind }
#[derive(Debug, Clone, Serialize, Deserialize)] pub struct Edge { pub from: u64, pub to: u64 }
#[derive(Debug, Clone, Serialize, Deserialize)] pub struct Graph { pub nodes: Vec<Node>, pub edges: Vec<Edge> }
#[derive(Debug, Clone)] pub struct CompiledGraph { pub graph: StableDiGraph<Node, ()>, pub id_to_index: std::collections::HashMap<u64, NodeIndex> }
