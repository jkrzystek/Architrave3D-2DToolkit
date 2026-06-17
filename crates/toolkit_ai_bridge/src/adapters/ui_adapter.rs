use std::sync::Arc;

use parking_lot::RwLock;
use serde_json::{json, Value};
use toolkit_ui::{ToolkitTheme, ThemeMode, WorkspaceLayout};

use crate::error::{BridgeError, BridgeResult};
use crate::provider::*;

/// AI bridge for UI theme and panel layout.
///
/// Exposes theme settings and panel visibility — NOT egui widget state or
/// render internals.
pub struct UiBridge {
    theme: Arc<RwLock<ToolkitTheme>>,
    layout: Arc<RwLock<WorkspaceLayout>>,
}

impl UiBridge {
    pub fn new(theme: Arc<RwLock<ToolkitTheme>>, layout: Arc<RwLock<WorkspaceLayout>>) -> Self {
        Self { theme, layout }
    }
}

impl AiProvider for UiBridge {
    fn namespace(&self) -> &str {
        "ui"
    }

    fn description(&self) -> &str {
        "UI theme and panel layout control. Switch dark/light mode, set accent \
         color, toggle panel visibility."
    }

    fn list_resources(&self) -> Vec<ResourceDescriptor> {
        vec![
            ResourceDescriptor::json(
                "ui://theme",
                "Theme Settings",
                "Current theme mode, accent color, viewport background, font sizes",
            ),
            ResourceDescriptor::json(
                "ui://panels",
                "Panel Layout",
                "All panels with visibility and closable state",
            ),
        ]
    }

    fn list_tools(&self) -> Vec<ToolDescriptor> {
        vec![
            ToolDescriptor::new(
                "ui.set_theme_mode",
                "Switch between dark and light theme",
                json!({
                    "type": "object",
                    "properties": {
                        "mode": {"type": "string", "enum": ["dark", "light"]},
                    },
                    "required": ["mode"]
                }),
            ),
            ToolDescriptor::new(
                "ui.set_accent_color",
                "Set the accent/highlight color (RGB 0-255)",
                json!({
                    "type": "object",
                    "properties": {
                        "r": {"type": "integer", "minimum": 0, "maximum": 255},
                        "g": {"type": "integer", "minimum": 0, "maximum": 255},
                        "b": {"type": "integer", "minimum": 0, "maximum": 255},
                    },
                    "required": ["r", "g", "b"]
                }),
            ),
            ToolDescriptor::new(
                "ui.toggle_panel",
                "Show or hide a panel by ID",
                json!({
                    "type": "object",
                    "properties": {
                        "panel_id": {"type": "string"},
                        "visible": {"type": "boolean"},
                    },
                    "required": ["panel_id", "visible"]
                }),
            ),
        ]
    }

    fn read_resource(&self, uri: &str) -> BridgeResult<ResourceContent> {
        match uri {
            "ui://theme" => {
                let theme = self.theme.read();
                ResourceContent::json(
                    uri,
                    &json!({
                        "mode": format!("{:?}", theme.mode),
                        "accent_color": theme.accent_color,
                        "viewport_background": theme.viewport_background,
                        "font_size_body": theme.font_size_body,
                        "font_size_heading": theme.font_size_heading,
                        "panel_rounding": theme.panel_rounding,
                    }),
                )
            }
            "ui://panels" => {
                let layout = self.layout.read();
                let panels: Vec<Value> = layout
                    .panels
                    .iter()
                    .map(|p| {
                        json!({
                            "id": p.id.0,
                            "title": p.title,
                            "visible": p.visible,
                            "closable": p.closable,
                        })
                    })
                    .collect();
                ResourceContent::json(uri, &json!({"panels": panels}))
            }
            _ => Err(BridgeError::ResourceNotFound(uri.into())),
        }
    }

    fn call_tool(&self, name: &str, args: Value) -> BridgeResult<ToolResult> {
        match name {
            "ui.set_theme_mode" => {
                let mode_str = args
                    .get("mode")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| BridgeError::InvalidArguments("missing 'mode'".into()))?;
                let mode = match mode_str {
                    "dark" => ThemeMode::Dark,
                    "light" => ThemeMode::Light,
                    _ => {
                        return Err(BridgeError::InvalidArguments(
                            "mode must be 'dark' or 'light'".into(),
                        ))
                    }
                };
                self.theme.write().mode = mode;
                ToolResult::success_json(&json!({"mode": mode_str}))
            }
            "ui.set_accent_color" => {
                let r = args.get("r").and_then(|v| v.as_u64()).unwrap_or(0) as u8;
                let g = args.get("g").and_then(|v| v.as_u64()).unwrap_or(0) as u8;
                let b = args.get("b").and_then(|v| v.as_u64()).unwrap_or(0) as u8;
                self.theme.write().accent_color = [r, g, b];
                ToolResult::success_json(&json!({"accent_color": [r, g, b]}))
            }
            "ui.toggle_panel" => {
                let panel_id = args
                    .get("panel_id")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| BridgeError::InvalidArguments("missing 'panel_id'".into()))?;
                let visible = args
                    .get("visible")
                    .and_then(|v| v.as_bool())
                    .ok_or_else(|| BridgeError::InvalidArguments("missing 'visible'".into()))?;
                let mut layout = self.layout.write();
                match layout.panels.iter_mut().find(|p| p.id.0 == panel_id) {
                    Some(panel) => {
                        panel.visible = visible;
                        ToolResult::success_json(&json!({
                            "panel_id": panel_id,
                            "visible": visible,
                        }))
                    }
                    None => Ok(ToolResult::error(format!("Panel '{panel_id}' not found"))),
                }
            }
            _ => Err(BridgeError::ToolNotFound(name.into())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_bridge() -> UiBridge {
        let theme = ToolkitTheme::default();
        let layout = WorkspaceLayout::default();
        UiBridge::new(Arc::new(RwLock::new(theme)), Arc::new(RwLock::new(layout)))
    }

    #[test]
    fn read_theme() {
        let bridge = make_bridge();
        let content = bridge.read_resource("ui://theme").unwrap();
        let v: Value = serde_json::from_str(&content.text).unwrap();
        assert_eq!(v["mode"], "Dark");
    }

    #[test]
    fn read_panels() {
        let bridge = make_bridge();
        let content = bridge.read_resource("ui://panels").unwrap();
        let v: Value = serde_json::from_str(&content.text).unwrap();
        assert!(v["panels"].as_array().unwrap().len() > 0);
    }

    #[test]
    fn switch_to_light() {
        let bridge = make_bridge();
        bridge
            .call_tool("ui.set_theme_mode", json!({"mode": "light"}))
            .unwrap();
        assert_eq!(bridge.theme.read().mode, ThemeMode::Light);
    }

    #[test]
    fn set_accent_color() {
        let bridge = make_bridge();
        bridge
            .call_tool("ui.set_accent_color", json!({"r": 255, "g": 100, "b": 50}))
            .unwrap();
        assert_eq!(bridge.theme.read().accent_color, [255, 100, 50]);
    }
}
