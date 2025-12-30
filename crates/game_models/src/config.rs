#![coverage(off)]

use std::fs::{read_to_string, write};
use std::path::Path;
use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use crate::key_utils::convert;
// =================================================================================================
//
//                                            Global
//
// =================================================================================================

#[derive(Resource, Deserialize, Serialize, Clone, Debug, Default)]
pub struct GlobalConfig {
    pub graphics_config: GraphicsConfig,
    pub input_config: InputConfig,
}

impl GlobalConfig {

    /// Loads a configuration file and deserializes it into the specified type.
    ///
    /// # Arguments
    /// - `path`: The file path of the configuration file to load.
    ///
    /// # Panics
    /// This function will panic if the file cannot be read or parsed correctly.
    ///
    /// # Returns
    /// - `T`: The deserialized configuration data.
    pub fn load<T: for<'de> Deserialize<'de>>(path: &str) -> T {
        let content = read_to_string(Path::new(path)).expect("Failed to read config file");
        toml::from_str(&content).expect("Failed to parse toml file")
    }

    /// Creates a new `GlobalConfig` instance and loads all configuration files.
    ///
    ///
    /// # Returns
    /// - `GlobalConfig`: A new instance with loaded configurations for game, graphics, input, and audio.
    pub fn new() -> Self {
        Self {
            graphics_config: Self::load("config/graphics.toml"),
            input_config: Self::load("config/input.toml"),
        }
    }

    /// Saves a specified file with his name.
    fn save<T: Serialize>(data: &T, path: &str) {
        let toml_string = toml::to_string_pretty(data).expect("Failed to serialize to TOML");
        write(Path::new(path), toml_string).expect("Failed to write config file");
    }

    /// Saves all known config files that found in config/ folder.
    /// This func used `GlobalConfig::save` for saving.
    pub fn save_all(&self) {
        Self::save(&self.graphics_config, "config/graphics.toml");
    }

}

// =================================================================================================
//
//                                            Graphics
//
// =================================================================================================

/// Serializable graphics configuration for windowing and rendering.
/// Stores human-readable strings (e.g., resolution `"1270x720"`, backend `"AUTO"`)
/// and toggles for fullscreen and vertical sync.
#[derive(Resource, Deserialize, Serialize, Clone, Debug)]
pub struct GraphicsConfig {
    /// Window resolution string in the form `"<width>x<height>"`.
    pub window_resolution: String,

    /// Whether to start in fullscreen mode.
    pub fullscreen: bool,
    /// Whether to enable vertical sync.
    pub vsync: bool,

    /// Requested graphics backend (e.g., `"AUTO"`, `"VULKAN"`, `"DX12"`, `"METAL"`).
    pub video_backend: String,
}

impl Default for GraphicsConfig {
    fn default() -> Self {
        Self {
            window_resolution: String::from("1270x720"),
            fullscreen: false,
            vsync: true,
            video_backend: String::from("AUTO")
        }
    }
}

impl GraphicsConfig {

    /// Parses and returns the configured window width in pixels.
    ///
    /// Falls back to `1280.0` if parsing fails.
    pub fn get_window_width(&self) -> u32 {
        let (width, _) = parse_resolution(self.window_resolution.as_str())
            .unwrap_or_else(|_| (1280, 720));
        width
    }

    /// Parses and returns the configured window height in pixels.
    ///
    /// Falls back to `720.0` if parsing fails.
    pub fn get_window_height(&self) -> u32 {
        let (_, height) = parse_resolution(self.window_resolution.as_str())
            .unwrap_or_else(|_| (1280, 720));
        height
    }
}

// =================================================================================================
//
//                                            Input
//
// =================================================================================================

/// Serializable input configuration mapping high-level actions to key names.
/// Stores human-readable key strings (e.g., "F1", "Space", "A") that are later
/// converted into engine `KeyCode`s at runtime.
#[derive(Resource, Deserialize, Serialize, Clone, Debug)]
pub struct InputConfig {
    /// Toggle developer inspector overlay.
    pub inspector: String,
    /// Toggle system information overlay.
    pub system_info: String,
    /// Toggle gizmo/boxes visualization.
    pub gizmos_boxen: String,

    /// Move character left.
    pub movement_left: String,
    /// Move character right.
    pub movement_right: String,
    /// Trigger jump action.
    pub movement_jump: String,

    /// Context-sensitive interaction (e.g., talk, use).
    pub interact: String,
}

impl Default for InputConfig {
    fn default() -> Self {
        Self {
            inspector: String::from("F1"),
            system_info: String::from("F3"),
            gizmos_boxen: String::from("F9"),

            movement_left: String::from("A"),
            movement_right: String::from("D"),
            movement_jump: String::from("Space"),

            interact: String::from("E")
        }
    }
}

impl InputConfig {
    pub fn get_inspector_key(&self) -> KeyCode {
        convert(self.inspector.as_str()).unwrap_or_else(|| KeyCode::F1)
    }

    pub fn get_system_info_key(&self) -> KeyCode {
        convert(self.system_info.as_str()).unwrap_or_else(|| KeyCode::F3)
    }

    pub fn get_gizmo_box_key(&self) -> KeyCode {
        convert(self.gizmos_boxen.as_str()).unwrap_or_else(|| KeyCode::F9)
    }

    pub fn get_move_left_key(&self) -> KeyCode {
        convert(self.movement_left.as_str()).unwrap_or_else(|| KeyCode::KeyA)
    }

    pub fn get_move_right_key(&self) -> KeyCode {
        convert(self.movement_right.as_str()).unwrap_or_else(|| KeyCode::KeyD)
    }

    pub fn get_jump_key(&self) -> KeyCode {
        convert(self.movement_jump.as_str()).unwrap_or_else(|| KeyCode::Space)
    }

    pub fn get_interact_key(&self) -> KeyCode {
        convert(self.interact.as_str()).unwrap_or_else(|| KeyCode::KeyE)
    }

}

// =================================================================================================
//
//                                         Internal Func
//
// =================================================================================================

/// Parses a resolution string in the form `"<width>x<height>"` (case-insensitive `x`)
/// into a pair of positive floating-point dimensions.
///
/// Accepts optional surrounding whitespace and trims each side. Width and
/// height must parse to numbers greater than zero; otherwise an error string
/// is returned.
///
/// # Parameters
/// * `s` - Input string like `"1280x720"` or `"1920X1080"`.
fn parse_resolution(s: &str) -> Result<(u32, u32), String> {
    let (w_str, h_str) = s
        .trim()
        .split_once(['x', 'X'])
        .ok_or_else(|| format!("Wrong Format: '{}'. Example z. B. 1280x720", s))?;

    let w: u32 = w_str.trim().parse()
        .map_err(|_| format!("Width is not a number: '{}'", w_str.trim()))?;
    let h: u32 = h_str.trim().parse()
        .map_err(|_| format!("Height is not a number: '{}'", h_str.trim()))?;

    if w <= 0 || h <= 0 {
        return Err("Width / Height needs a positive number like > 0".into());
    }
    Ok((w, h))
}