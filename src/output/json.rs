//! JSON export for layout results.

use crate::models::{LayoutResult, PhotoPlacement};
use anyhow::{Context, Result};
use serde::{Serialize, Deserialize};
use std::path::Path;

/// Serializable layout result for JSON export.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayoutJson {
    pub canvas: CanvasJson,
    pub placements: Vec<PlacementJson>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanvasJson {
    pub width_mm: f64,
    pub height_mm: f64,
    pub beta_mm: f64,
    pub bleed_mm: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlacementJson {
    pub photo_idx: u16,
    pub photo_path: String,
    pub x_mm: f64,
    pub y_mm: f64,
    pub width_mm: f64,
    pub height_mm: f64,
}

/// Exports a layout result to a JSON file with photo paths.
pub fn export_json(layout: &LayoutResult, photo_paths: &[String], output_path: &Path) -> Result<()> {
    let json = layout_to_json(layout, photo_paths);
    let json_str = serde_json::to_string_pretty(&json)
        .context("Failed to serialize layout to JSON")?;
    
    std::fs::write(output_path, json_str)
        .with_context(|| format!("Failed to write JSON to {:?}", output_path))?;
    
    Ok(())
}

/// Converts a layout result to JSON-serializable format with photo paths.
fn layout_to_json(layout: &LayoutResult, photo_paths: &[String]) -> LayoutJson {
    LayoutJson {
        canvas: CanvasJson {
            width_mm: layout.canvas.width,
            height_mm: layout.canvas.height,
            beta_mm: layout.canvas.beta,
            bleed_mm: layout.canvas.bleed,
        },
        placements: layout
            .placements
            .iter()
            .map(|p| placement_to_json(p, photo_paths))
            .collect(),
    }
}

fn placement_to_json(placement: &PhotoPlacement, photo_paths: &[String]) -> PlacementJson {
    let photo_path = photo_paths
        .get(placement.photo_idx as usize)
        .cloned()
        .unwrap_or_else(|| format!("unknown_{}", placement.photo_idx));
    
    PlacementJson {
        photo_idx: placement.photo_idx,
        photo_path,
        x_mm: placement.x,
        y_mm: placement.y,
        width_mm: placement.w,
        height_mm: placement.h,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::Canvas;

    #[test]
    fn test_layout_to_json() {
        let canvas = Canvas::new(1000.0, 800.0, 5.0, 3.0);
        let placements = vec![
            PhotoPlacement {
                photo_idx: 0,
                x: 10.0,
                y: 20.0,
                w: 300.0,
                h: 200.0,
            },
            PhotoPlacement {
                photo_idx: 1,
                x: 320.0,
                y: 20.0,
                w: 400.0,
                h: 200.0,
            },
        ];
        
        let layout = LayoutResult::new(placements, canvas);
        let photo_paths = vec![
            "photo1.jpg".to_string(),
            "photo2.jpg".to_string(),
        ];
        let json = layout_to_json(&layout, &photo_paths);
        
        assert_eq!(json.placements.len(), 2);
        assert_eq!(json.canvas.width_mm, 1000.0);
        assert_eq!(json.canvas.beta_mm, 5.0);
        assert_eq!(json.placements[0].photo_path, "photo1.jpg");
        assert_eq!(json.placements[1].photo_path, "photo2.jpg");
    }

    #[test]
    fn test_json_roundtrip() {
        let canvas = Canvas::new(1000.0, 800.0, 5.0, 3.0);
        let placements = vec![
            PhotoPlacement {
                photo_idx: 0,
                x: 10.0,
                y: 20.0,
                w: 300.0,
                h: 200.0,
            },
        ];
        
        let layout = LayoutResult::new(placements, canvas);
        let photo_paths = vec!["photo1.jpg".to_string()];
        let json = layout_to_json(&layout, &photo_paths);
        
        // Serialize and deserialize
        let json_str = serde_json::to_string(&json).unwrap();
        let json_back: LayoutJson = serde_json::from_str(&json_str).unwrap();
        
        assert_eq!(json_back.placements.len(), json.placements.len());
        assert_eq!(json_back.canvas.width_mm, json.canvas.width_mm);
        assert_eq!(json_back.placements[0].photo_path, "photo1.jpg");
    }
}
