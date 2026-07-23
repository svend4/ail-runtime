//! Step 5: VizGraph — visual style from ConnectivityDegree
//!
//! Turns the relation graph into a simple renderable description.
//! No real GUI yet: produces a structured scene that a front-end
//! or terminal renderer can consume.

use ail_mvp::connectivity::*;
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Visual primitives
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: f32,
}

impl Color {
    pub fn rgba(r: u8, g: u8, b: u8, a: f32) -> Self {
        Self { r, g, b, a }
    }
}

#[derive(Clone, Debug)]
pub enum LineStyle {
    Dashed,
    Solid,
    Bold,
}

#[derive(Clone, Debug)]
pub struct VizNode {
    pub id: NodeId,
    pub label: String,
    pub x: f64,
    pub y: f64,
    pub radius: f64,
}

#[derive(Clone, Debug)]
pub struct VizEdge {
    pub from: NodeId,
    pub to: NodeId,
    pub label: String,
    pub degree: ConnectivityDegree,
    pub saturation: f64,
    pub weight: f32,
    pub color: Color,
    pub width: f32,
    pub style: LineStyle,
}

#[derive(Clone, Debug)]
pub struct VizScene {
    pub nodes: Vec<VizNode>,
    pub edges: Vec<VizEdge>,
    pub title: String,
}

// ---------------------------------------------------------------------------
// Style mapping (degree → visual)
// ---------------------------------------------------------------------------

fn style_for_degree(d: ConnectivityDegree, saturation: f64, weight: f32) -> (Color, f32, LineStyle) {
    // base width scales a little with weight
    let w = (1.0 + weight * 0.3).clamp(1.0, 4.0);

    match d {
        ConnectivityDegree::D1 => {
            // thin, dashed, low alpha, cool grey-blue
            (
                Color::rgba(140, 160, 180, 0.45),
                w * 0.7,
                LineStyle::Dashed,
            )
        }
        ConnectivityDegree::D2 => {
            // normal solid, medium alpha, amber / teal depending on saturation
            let t = saturation.clamp(0.0, 1.0) as f32;
            let r = (80.0 + 100.0 * t) as u8;
            let g = (160.0 - 40.0 * t) as u8;
            let b = (140.0 - 60.0 * t) as u8;
            (Color::rgba(r, g, b, 0.85), w, LineStyle::Solid)
        }
        ConnectivityDegree::D3 => {
            // thick, bright, high alpha, red-orange
            (
                Color::rgba(220, 70, 50, 0.95),
                w * 1.4,
                LineStyle::Bold,
            )
        }
    }
}

// ---------------------------------------------------------------------------
// Builder
// ---------------------------------------------------------------------------

pub struct VizGraph;

impl VizGraph {
    /// Build a scene from a connectivity Graph and optional name map.
    pub fn from_graph(
        graph: &Graph,
        labels: &HashMap<NodeId, String>,
        title: &str,
    ) -> VizScene {
        // trivial circular layout
        let node_ids: Vec<NodeId> = {
            let mut s = std::collections::HashSet::new();
            for e in &graph.edges {
                s.insert(e.from);
                s.insert(e.to);
            }
            // also isolated nodes if any were registered via labels
            for id in labels.keys() {
                s.insert(*id);
            }
            s.into_iter().collect()
        };

        let n = node_ids.len().max(1) as f64;
        let mut nodes = Vec::new();
        for (i, id) in node_ids.iter().enumerate() {
            let angle = std::f64::consts::TAU * (i as f64) / n;
            let label = labels
                .get(id)
                .cloned()
                .unwrap_or_else(|| format!("n{}", id.0));
            nodes.push(VizNode {
                id: *id,
                label,
                x: angle.cos(),
                y: angle.sin(),
                radius: 0.12,
            });
        }

        let mut edges = Vec::new();
        for e in &graph.edges {
            let (color, width, style) = style_for_degree(e.degree, e.saturation, e.weight);
            edges.push(VizEdge {
                from: e.from,
                to: e.to,
                label: format!("{:?}", e.symbol),
                degree: e.degree,
                saturation: e.saturation,
                weight: e.weight,
                color,
                width,
                style,
            });
        }

        VizScene {
            nodes,
            edges,
            title: title.to_string(),
        }
    }

    /// Terminal-friendly dump of the scene.
    pub fn print_scene(scene: &VizScene) {
        println!("=== VizScene: {} ===", scene.title);
        println!("Nodes:");
        for n in &scene.nodes {
            println!(
                "  {:?} `{}` at ({:.2}, {:.2})",
                n.id, n.label, n.x, n.y
            );
        }
        println!("Edges:");
        for e in &scene.edges {
            println!(
                "  {:?} -> {:?} | {:?} | sat={:.2} w={:.1} | style={:?} width={:.1} rgba=({},{},{},{:.2})",
                e.from,
                e.to,
                e.degree,
                e.saturation,
                e.weight,
                e.style,
                e.width,
                e.color.r,
                e.color.g,
                e.color.b,
                e.color.a
            );
        }
        println!();
    }
}
