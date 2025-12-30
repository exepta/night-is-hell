#![feature(coverage_attribute)]

use std::env;
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use bevy::log::{BoxedLayer, Level, LogPlugin};
use bevy::prelude::*;
use bevy::render::render_resource::WgpuFeatures;
use bevy::render::RenderPlugin;
use bevy::render::settings::{Backends, RenderCreation, WgpuSettings};
use bevy::window::{PresentMode, WindowMode, WindowResolution};
use bevy_inspector_egui::bevy_egui::EguiPlugin;
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use chrono::Utc;
use dotenvy::dotenv;
use tracing_subscriber::fmt::writer::BoxMakeWriter;
use tracing_subscriber::Layer;
use game_models::config::GlobalConfig;
use game_models::debug::{BuildInfo, WorldInspectorState};
use game_models::states::AppState;
use crate::manager::ManagerPlugin;

/// Helper struct to insert a start log entry when logging is initialized.
///
/// When this struct is dropped, it writes a separator message to the log file
/// to indicate when logging has started.
struct StartLogText {
    file: Arc<Mutex<File>>, // Shared reference to the log file
}

impl Drop for StartLogText {

    #[coverage(off)]
    fn drop(&mut self) {
        let mut file = self.file.lock().unwrap();
        let _ = writeln!(
            file,
            "\n====================================== [ Start ] ======================================\n"
        );
        let _ = file.flush();
    }
}

/// Application entry point for debug builds.
/// Initializes logging, loads configuration, creates the Bevy app, and runs the core client.
///
/// This function is only included in debug builds.
#[cfg(debug_assertions)]
#[coverage(off)]
fn main() {
    let config = GlobalConfig::new();
    let mut app = App::new();
    init_bevy_app(&mut app, &config);
}

/// Application entry point for release builds.
/// Initializes configuration, creates the Bevy app, and runs the core client.
///
/// This function is only included in release builds.
#[cfg(not(debug_assertions))]
#[coverage(off)]
fn main() {
    let config = GlobalConfig::new();
    let mut app = App::new();
    init_bevy_app(&mut app, &config);
}

/// Initializes core Bevy app plugins and logging settings.
///
/// # Parameters
/// - `app`: A mutable reference to the [`App`] instance.
/// - `config`: [`GlobalConfig`] containing window configuration.
#[coverage(off)]
fn init_bevy_app(app: &mut App, config: &GlobalConfig) {
    let build = BuildInfo {
        app_name: "Night Is Hell",
        app_version: env!("CARGO_PKG_VERSION"),
        bevy_version: "0.17.3",
    };

    app
        .insert_resource(config.clone())
        .insert_resource(build)
        .add_plugins(DefaultPlugins.set(
            WindowPlugin {
                primary_window: Some(Window {
                    title: String::from("Gear Born"),
                    mode: if config.graphics_config.fullscreen { WindowMode::BorderlessFullscreen(MonitorSelection::Primary) } else { WindowMode::Windowed }, //BorderlessFullscreen(MonitorSelection::Primary)
                    resolution: WindowResolution::new(
                        config.graphics_config.get_window_width(),
                        config.graphics_config.get_window_height()),
                    present_mode: PresentMode::AutoVsync,
                    ..default()
                }),
                ..default()
            }
        ).set(
            RenderPlugin {
                render_creation: RenderCreation::Automatic(create_gpu_settings(&config.graphics_config.video_backend)),
                ..default()
            }
        ).set(ImagePlugin::default_nearest()
        ).set(LogPlugin {
            level: Level::DEBUG,
            filter: load_log_env_filter(),
            custom_layer: log_file_appender,
            ..default()
        }))
        .insert_resource(ClearColor(Color::Srgba(Srgba::rgb_u8(20, 25,27))))
        .init_state::<AppState>()
        .insert_resource(WorldInspectorState(false))
        .add_plugins(EguiPlugin::default())
        .add_plugins(WorldInspectorPlugin::default().run_if(check_world_inspector_state))
        .add_plugins(ManagerPlugin)
        .add_systems(Update, init_app_finish.run_if(in_state(AppState::AppInit).and(resource_exists::<GlobalConfig>)))
        .run();
}

/// After finish creation of the app `init_bevy_app` this function will
/// set the current [`AppState`] to `AppState::PreLoad`.
#[coverage(off)]
fn init_app_finish(mut next_state: ResMut<NextState<AppState>>) {
    info!("Finish initializing app...");
    next_state.set(AppState::Preload);
}

/// Creates GPU settings for rendering.
///
/// The settings include enabling Vulkan as the rendering backend and enabling
/// the `POLYGON_MODE_LINE` feature for wireframe rendering.
///
/// # Returns
/// A configured [`WgpuSettings`] instance.
///
/// # Example
/// ```rust
/// let gpu_settings = create_gpu_settings();
/// ```
#[coverage(off)]
fn create_gpu_settings(backend_str: &str) -> WgpuSettings {
    let backend = match backend_str {
        "auto" | "AUTO" | "primary" | "PRIMARY" => Some(Backends::PRIMARY),
        "vulkan" | "VULKAN" => Some(Backends::VULKAN),
        "dx12" | "DX12" => Some(Backends::DX12),
        "metal" | "METAL" => Some(Backends::METAL),
        other => {
            eprintln!("Unknown backend '{}', falling back to PRIMARY", other);
            Some(Backends::PRIMARY)
        }
    };

    WgpuSettings {
        features: if cfg!(debug_assertions) { WgpuFeatures::POLYGON_MODE_LINE } else { WgpuFeatures::empty() },
        backends: backend,
        ..default()
    }
}


/// Checks whether the World Inspector UI is currently enabled or not.
///
/// This function simply checks the state of the `WorldInspectorState`
/// and returns a boolean indicating whether the World Inspector UI is visible.
///
/// # Arguments
///
/// * `world_inspector_state`: A reference to the state of the world inspector UI.
///
/// # Returns
///
/// * `true` if the World Inspector UI is visible (enabled).
/// * `false` if the World Inspector UI is not visible (disabled).
#[coverage(off)]
fn check_world_inspector_state(
    world_inspector_state: Res<WorldInspectorState>,
) -> bool {
    world_inspector_state.0
}

/// Initializes a log file appender for the application.
///
/// This function creates a `logs` directory if it does not exist and generates a log file
/// with a timestamped name in the format `bevy-DD-MM-YYYY.log`. It then sets up a
/// logging layer that writes log messages to this file.
///
/// # Parameters
/// - `_app`: A mutable reference to the Bevy `App`. (Currently unused)
///
/// # Returns
/// - `Some(BoxedLayer)`: If the log file was successfully created and opened.
/// - `None`: If there was an error creating the log directory or opening the file.
///
/// # Logging Details
/// - The log file is set up to append new logs.
/// - ANSI formatting is disabled for better readability in plain text files.
/// - The log writer ensures proper synchronization using `Arc<Mutex<File>>`.
#[coverage(off)]
fn log_file_appender(_app: &mut App) -> Option<BoxedLayer> {
    let log_dir = PathBuf::from("logs");
    if let Err(e) = std::fs::create_dir_all(&log_dir) {
        eprintln!("Failed to create log directory: {}", e);
        return None;
    }

    let timestamp = Utc::now().format("bevy-%d-%m-%Y.log").to_string();
    let log_path = log_dir.join(timestamp);

    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path)
        .ok()?;

    let file_arc = Arc::new(Mutex::new(file));

    let _shutdown_logger = StartLogText {
        file: Arc::clone(&file_arc),
    };

    let writer = BoxMakeWriter::new(move || {
        let file = file_arc.lock().unwrap().try_clone().expect("Failed to clone log file handle");
        Box::new(file) as Box<dyn Write + Send>
    });

    Some(Box::new(tracing_subscriber::fmt::layer()
        .with_ansi(false)
        .with_writer(writer)
        .boxed()
    ))
}

/// Loads the `LOG_ENV_FILTER` environment variable from a `.env` file and returns it as a `String`.
///
/// This function uses the `dotenv` crate to load environment variables from a `.env` file in the project root (if present).
/// If the `LOG_ENV_FILTER` variable is not set, it defaults to `"error"`.
///
/// # Returns
/// A `String` containing the log filter settings to use for logging frameworks (e.g., tracing).
///
/// # Example
/// ```rust
/// let log_filter = load_log_env_filter();
/// // Pass `log_filter` to your logging/tracing setup
/// ```
///
#[coverage(off)]
fn load_log_env_filter() -> String {
    dotenv().ok();
    let env = env::var("LOG_ENV_FILTER").unwrap_or_else(|_| "error".to_string());
    env.to_string()
}

// =================================================================================================
//
//                                            Manager
//
// =================================================================================================

mod manager {
    use bevy::light::DirectionalLightShadowMap;
    use bevy::prelude::*;
    use bevy_rapier3d::prelude::*;
    use game_logic::GameLogicPlugin;
    use game_models::config::GlobalConfig;
    use game_models::debug::WorldInspectorState;
    use game_models::GameCorePlugin;
    use game_ui::GameUiPlugin;

    pub struct ManagerPlugin;

    impl Plugin for ManagerPlugin {

        #[coverage(off)]
        fn build(&self, app: &mut App) {
            app.add_plugins(RapierPhysicsPlugin::<NoUserData>::default());
            app.add_plugins(RapierDebugRenderPlugin {
                enabled: false,
                ..default()
            });

            app.add_plugins((GameCorePlugin, GameLogicPlugin, GameUiPlugin));

            app.add_systems(Startup, setup_shadow_map);
            app.add_systems(Update, (toggle_world_inspector, toggle_debug_system));
        }
    }

    #[coverage(off)]
    fn setup_shadow_map(mut commands: Commands) {
        commands.insert_resource(DirectionalLightShadowMap { size: 1024 });
    }

    #[coverage(off)]
    fn toggle_world_inspector(
        mut debug_context: ResMut<WorldInspectorState>,
        keyboard: ResMut<ButtonInput<KeyCode>>,
        global_config: Res<GlobalConfig>
    ) {
        let key = global_config.input_config.get_inspector_key();
        if keyboard.just_pressed(key) {
            debug_context.0 = !debug_context.0;
        }
    }

    #[coverage(off)]
    pub fn toggle_debug_system(
        mut debug_context: ResMut<DebugRenderContext>,
        keyboard: ResMut<ButtonInput<KeyCode>>,
        global_config: Res<GlobalConfig>
    ) {
        let key = global_config.input_config.get_gizmo_box_key();
        if keyboard.just_pressed(key) {
            debug_context.enabled = !debug_context.enabled
        }
    }

}
