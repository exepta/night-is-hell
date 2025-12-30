use bevy::diagnostic::{DiagnosticsStore, EntityCountDiagnosticsPlugin, FrameTimeDiagnosticsPlugin};
use bevy::prelude::{in_state, App, IntoScheduleConfigs, OnEnter, Plugin, Res, ResMut, Time, Update};
use bevy::render::renderer::RenderAdapterInfo;
use sysinfo::{CpuRefreshKind, MemoryRefreshKind, Pid, ProcessesToUpdate, RefreshKind, System};
use game_models::config::GlobalConfig;
use game_models::debug::{BuildInfo, DebugOverlayState, DebugSnapshot, SysStats};
use game_models::states::AppState;
use game_models::v_ram_detection::{detect_v_ram_best_effort, fmt_bytes};

pub struct DebugLogicComponent;

impl Plugin for DebugLogicComponent {
    fn build(&self, app: &mut App) {
        app.add_plugins((FrameTimeDiagnosticsPlugin::default(), EntityCountDiagnosticsPlugin::default()));
        app
            .init_resource::<DebugSnapshot>()
            .init_resource::<DebugOverlayState>()
            .init_resource::<SysStats>();

        app.add_systems(
            OnEnter(AppState::Preload), internal_sys_info.run_if(in_state(AppState::Preload))
        );

        app.add_systems(Update, poll_sys_info);
        app.add_systems(Update,
                        (
                            snap_perf,
                            snap_build,
                            snap_v_ram,
                            snap_cpu_brand
                        )
                            .chain()
                            .run_if(in_state(AppState::Preload)));
    }

}

/// Initializes a system/process stats collection for the current PID and stores an initial
/// snapshot (app memory, app CPU %) into `SysStats`. Creates a fresh `sysinfo::System`
/// with CPU and memory refresh enabled.
///
/// # Parameters
/// * `sys_stats` - Mutable stats resource to populate with the initial snapshot and system handle.
#[coverage(off)]
fn internal_sys_info(
    mut sys_stats: ResMut<SysStats>
) {
    let mut system = System::new_with_specifics(
        RefreshKind::default()
            .with_memory(MemoryRefreshKind::default())
            .with_cpu(CpuRefreshKind::default())
    );

    let p_id = Pid::from_u32(std::process::id());
    system.refresh_cpu_all();
    system.refresh_processes(ProcessesToUpdate::Some(&[p_id]), true);

    let (app_mem_bytes, app_cpu) = system.process(p_id)
        .map(|process| (process.memory(), process.cpu_usage()))
        .unwrap_or((0, 0.0));

    sys_stats.app_mem_bytes = app_mem_bytes;
    sys_stats.app_cpu_percent = app_cpu;
    sys_stats.sys = system;
}

/// Periodically refreshes OS-level CPU and process metrics and writes normalized
/// app CPU %, total CPU %, and app memory into `SysStats`. Uses a timer within
/// `SysStats` to rate-limit updates.
///
/// # Parameters
/// * `time` - Global time used to tick the internal sampling timer.
/// * `sys_stats` - Mutable stats resource holding the system handle and accumulators.
#[coverage(off)]
fn poll_sys_info(time: Res<Time>, mut sys_stats: ResMut<SysStats>, debug_state: Res<DebugOverlayState>) {
    if !debug_state.0 {
        return;
    }

    if sys_stats.timer.tick(time.delta()).just_finished() {
        let pid = Pid::from_u32(std::process::id());

        sys_stats.sys.refresh_cpu_all();
        sys_stats.sys.refresh_processes(ProcessesToUpdate::Some(&[pid]), false);

        let cpu_all_percent = sys_stats.sys.global_cpu_usage();

        let (app_mem_bytes, app_cpu_raw) = match sys_stats.sys.process(pid) {
            Some(p) => (p.memory(), p.cpu_usage()),
            None => (0, 0.0),
        };

        let logical = sys_stats.sys.cpus().len().max(1) as f32;
        let app_cpu_norm = (app_cpu_raw / logical).clamp(0.0, 100.0);

        sys_stats.cpu_all_percent = cpu_all_percent;
        sys_stats.app_mem_bytes = app_mem_bytes;
        sys_stats.app_cpu_percent = app_cpu_norm;
    }
}

/// Copies frame timing diagnostics (FPS) and current OS metrics (CPU %, memory)
/// from `DiagnosticsStore` and `SysStats` into the `DebugSnapshot` for UI rendering.
///
/// # Parameters
/// * `diag` - Bevy diagnostics store, read for smoothed FPS.
/// * `stats` - Latest normalized CPU/memory metrics.
/// * `snap` - Mutable snapshot written for the overlay.
#[coverage(off)]
fn snap_perf(diag: Res<DiagnosticsStore>, stats: Res<SysStats>, mut snap: ResMut<DebugSnapshot>, debug_state: Res<DebugOverlayState>) {
    if !debug_state.0 {
        return;
    }
    snap.fps = diag.get(&FrameTimeDiagnosticsPlugin::FPS)
        .and_then(|d| d.smoothed()).unwrap_or(0.0) as f32;
    snap.cpu_all_percent = stats.cpu_all_percent;
    snap.app_cpu_percent = stats.app_cpu_percent;
    snap.app_mem_bytes = stats.app_mem_bytes;
}

/// Populates build strings and graphics backend info for the overlay and record
/// relevant hotkey labels from global configuration.
///
/// # Parameters
/// * `build` - Optional build metadata (app name/version, Bevy version).
/// * `backend` - Active render adapter information (name/backend).
/// * `snap` - Mutable snapshot receiving build/backend fields.
/// * `global_config` - Source of hotkey binding labels.
#[coverage(off)]
fn snap_build(
    build: Option<Res<BuildInfo>>,
    backend: Res<RenderAdapterInfo>,
    mut snap: ResMut<DebugSnapshot>,
    global_config: Res<GlobalConfig>,
    debug_state: Res<DebugOverlayState>
) {
    if !debug_state.0 {
        return;
    }

    let (app_name, app_ver, bevy_ver) = if let Some(b) = build {
        (b.app_name, b.app_version, b.bevy_version)
    } else { ("<app>", "?", "0.17.3") };

    snap.app_name = app_name;
    snap.app_ver = app_ver;
    snap.bevy_ver = bevy_ver;
    snap.backend_name = backend.name.clone();
    snap.backend_str = match backend.backend.to_str() {
        "vulkan" => "Vulkan",
        "gl" => "OpenGL",
        "metal" => "Metal",
        "dx12" | "DX12" => "DirectX12",
        "dx11" | "DX11" => "DirectX11",
        _ => "Unknown",
    };
    snap.key_debug_info = global_config.input_config.system_info.clone();
    snap.key_gizmos = global_config.input_config.gizmos_boxen.clone();
}

/// Fills in a human-readable CPU brand/model string in the snapshot once, based
/// on the first non-empty CPU brand/name reported by `sysinfo`.
///
/// # Parameters
/// * `stats` - Access to the underlying `sysinfo::System`.
/// * `snap` - Mutable snapshot to receive the CPU brand string.
#[coverage(off)]
fn snap_cpu_brand(stats: Res<SysStats>, mut snap: ResMut<DebugSnapshot>, debug_state: Res<DebugOverlayState>) {
    if !debug_state.0 {
        return;
    }

    if !snap.cpu_brand.is_empty() {
        return;
    }

    let brand = stats
        .sys
        .cpus()
        .iter()
        .map(|c| c.brand().trim())
        .find(|b| !b.is_empty())
        .map(|s| s.to_string())
        .or_else(|| {
            stats
                .sys
                .cpus()
                .first()
                .map(|c| c.name().trim().to_string())
        })
        .unwrap_or_else(|| "Unknown CPU".to_string());

    snap.cpu_brand = brand;
}

/// Attempts to detect available V-RAM and writes a formatted label into the snapshot.
/// Falls back to `"n/a"` when detection fails.
///
/// # Parameters
/// * `snap` - Mutable snapshot to receive the V-RAM label.
#[coverage(off)]
fn snap_v_ram(mut snap: ResMut<DebugSnapshot>, debug_state: Res<DebugOverlayState>) {
    if !debug_state.0 {
        return;
    }

    if let Some(info) = detect_v_ram_best_effort() {
        snap.v_ram_label = format!(
            "{} ({})",
            fmt_bytes(info.bytes),
            info.source
        );
    } else {
        snap.v_ram_label = "n/a".to_string();
    }
}

/// Returns whether the debug overlay is currently visible.
/// Safe to call when the resource is absent; defaults to `false`.
///
/// # Parameters
/// * `state` - Optional `DebugOverlayState` resource to read the `show` flag from.
#[coverage(off)]
pub fn overlay_visible(state: Option<Res<DebugOverlayState>>) -> bool {
    state.map_or(false, |s| s.0)
}