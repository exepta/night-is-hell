use bevy::prelude::*;

/// Represents the state of the World Inspector UI.
///
/// This resource holds a single boolean value indicating whether the World Inspector UI
/// is currently visible or hidden. The state can be toggled by user input (e.g., a key press),
/// and this struct is used to track the visibility of the World Inspector in the application.
///
/// The `WorldInspectorState` is initialized to `false` (hidden) by default.
///
/// # Fields
///
/// * `0`: A boolean value that represents the visibility of the World Inspector UI.
///   - `true`: The World Inspector is visible.
///   - `false`: The World Inspector is hidden.
#[derive(Resource, Default, Debug)]
pub struct WorldInspectorState(pub bool);

#[derive(Resource, Clone)]
pub struct BuildInfo {
    pub app_name: &'static str,
    pub app_version: &'static str,
    pub bevy_version: &'static str,
}

/// Runtime state for a simple on-screen debug overlay (e.g., FPS, system stats).
///
/// The overlay is created lazily: `root` and `text` are populated once the
/// corresponding UI entities are spawned.
#[derive(Resource, Default)]
pub struct DebugOverlayState(pub bool);

/// Periodically sampled system/application performance metrics.
///
/// The underlying collector is `sysinfo::System` (`sys` field). Values are
/// updated on a repeating timer (`timer`) and are expected to be in:
/// - `cpu_percent`: global CPU usage in percent (0.0–100.0).
/// - `app_cpu_percent`: current process CPU usage in percent (0.0–100.0).
/// - `app_mem_bytes`: current process memory usage in **bytes**.
///
/// **Usage notes:**
/// - `System::new()` does not populate data; call `refresh_*` (e.g. `refresh_all`,
///   `refresh_cpu`, `refresh_processes`) before reading values.
/// - 'Timer' controls the sampling cadence; by default, it ticks every 0.5 s.
#[derive(Resource)]
pub struct SysStats {
    /// Sys_info handle used to query system and process metrics.
    pub sys: sysinfo::System,
    /// Global CPU utilization in percent.
    pub cpu_all_percent: f32,
    /// CPU utilization of the current application/process in percent.
    pub app_cpu_percent: f32,
    /// Memory usage of the current application/process in bytes.
    pub app_mem_bytes: u64,
    /// Repeating timer determining how often metrics are refreshed.
    pub timer: Timer,
}

impl Default for SysStats {
    /// Creates a `SysStats` with an empty `System` handle and a 0.5 s sampling interval.
    ///
    /// After construction, call the appropriate `sys.refresh_*` methods on each
    /// timer tick before reading the metrics.
    fn default() -> Self {
        Self {
            sys: sysinfo::System::new(),
            cpu_all_percent: 0.0,
            app_cpu_percent: 0.0,
            app_mem_bytes: 0,
            timer: Timer::from_seconds(0.5, TimerMode::Repeating),
        }
    }
}

/// Snapshot of runtime diagnostics and labels used by the on-screen debug
/// overlay. Captures performance metrics, player/camera info, build strings,
/// and hotkey hints for UI rendering.
#[derive(Resource, Default)]
pub struct DebugSnapshot {
    // Numbers
    /// Current frames per second.
    pub fps: f32,
    /// Total CPU usage across all cores (%).
    pub cpu_all_percent: f32,
    /// This application's CPU usage (%).
    pub app_cpu_percent: f32,
    /// This application's resident memory in bytes.
    pub app_mem_bytes: u64,
    /// Human-readable V-RAM usage/label for display.
    pub v_ram_label: String,

    // Game Infos
    /// Player world position used for HUD display.
    pub player_pos: Vec3,
    /// Current character name.
    pub character_name: String,

    // Build / Config
    /// Application name.
    pub app_name: &'static str,
    /// Application version string.
    pub app_ver: &'static str,
    /// Bevy version string.
    pub bevy_ver: &'static str,
    /// Active graphics backend name (e.g., Vulkan/Metal/DX12).
    pub backend_name: String,
    /// CPU brand/model string.
    pub cpu_brand: String,
    /// Short backend label used in UI.
    pub backend_str: &'static str,

    // Hotkeys (for UI)
    /// Key binding to toggle the debug overlay.
    pub key_debug_info: String,
    /// Key binding to toggle gizmos.
    pub key_gizmos: String,
}
