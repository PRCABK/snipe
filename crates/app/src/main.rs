#![windows_subsystem = "windows"]

mod controller;
mod hotkey;
mod pin_manager;
mod single_instance;
mod tray;

use config::AppConfig;
use domain::HotkeyAction;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::mpsc::unbounded_channel;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

fn main() -> anyhow::Result<()> {
    // 1. Initialize Tracing logger
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));
    tracing_subscriber::registry()
        .with(filter)
        .with(tracing_subscriber::fmt::layer().with_writer(std::io::stderr))
        .init();

    info!("Starting Snipe Application v{}", env!("CARGO_PKG_VERSION"));

    // 2. Handle command line flags (e.g. update shutdown or second instance)
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--shutdown-for-update") {
        info!("Sending shutdown-for-update command to primary instance...");
        single_instance::send_command_to_primary("SHUTDOWN_FOR_UPDATE");
        return Ok(());
    }

    // 3. Single instance enforcement
    let _instance_guard = match single_instance::try_acquire_instance() {
        Some(guard) => guard,
        None => {
            info!("Existing instance detected. Forwarding command to show settings...");
            single_instance::send_command_to_primary("OPEN_SETTINGS");
            return Ok(());
        }
    };

    // 4. Setup Tokio runtime for background tasks
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;
    let _enter = rt.enter();

    // 5. Setup channels & cancellation
    let (ipc_tx, mut ipc_rx) = unbounded_channel::<String>();
    let (sys_tx, mut sys_rx) = unbounded_channel::<tray::SystemMessage>();
    let shutdown_flag = Arc::new(AtomicBool::new(false));

    single_instance::start_ipc_server(ipc_tx, shutdown_flag.clone());

    // 6. Native Win32 Message Window & Tray
    #[cfg(windows)]
    let msg_window = tray::Win32MessageWindow::new(sys_tx.clone())
        .map_err(|e| anyhow::anyhow!("Failed to create Win32 message window: {}", e))?;

    // 7. Load config & register global hotkeys
    let cfg = AppConfig::load();

    #[cfg(windows)]
    {
        hotkey::register_global_hotkey(
            msg_window.hwnd,
            hotkey::HOTKEY_ID_SCREENSHOT,
            &cfg.hotkeys.screenshot,
        );
        hotkey::register_global_hotkey(
            msg_window.hwnd,
            hotkey::HOTKEY_ID_COLOR,
            &cfg.hotkeys.color_picker,
        );
        hotkey::register_global_hotkey(
            msg_window.hwnd,
            hotkey::HOTKEY_ID_LONGSHOT,
            &cfg.hotkeys.longshot,
        );
    }

    // 8. Keep all Slint objects on the UI thread. AppController is deliberately
    // not Send because it owns Slint windows and Rc-backed UI state.
    let controller = Rc::new(controller::AppController::new());

    // 9. Drain native/IPC events from the Slint thread. Moving AppController
    // into tokio::spawn would require its Slint windows to be Send.
    #[cfg(windows)]
    let notification_hwnd = msg_window.hwnd;
    let ctrl_for_events = controller.clone();
    let event_timer = slint::Timer::default();
    event_timer.start(
        slint::TimerMode::Repeated,
        std::time::Duration::from_millis(10),
        move || {
            while let Ok(msg) = sys_rx.try_recv() {
                match msg {
                    tray::SystemMessage::Hotkey(action) => match action {
                        HotkeyAction::Screenshot => ctrl_for_events.trigger_screenshot(),
                        HotkeyAction::ColorPicker => ctrl_for_events.trigger_color_picker(),
                        HotkeyAction::Settings => ctrl_for_events.open_settings(),
                        HotkeyAction::Longshot | HotkeyAction::Pin => {
                            ctrl_for_events.trigger_screenshot()
                        }
                    },
                    tray::SystemMessage::TrayAction(menu_id) => match menu_id {
                        tray::TRAY_MENU_SCREENSHOT => ctrl_for_events.trigger_screenshot(),
                        tray::TRAY_MENU_COLOR => ctrl_for_events.trigger_color_picker(),
                        tray::TRAY_MENU_LONGSHOT => ctrl_for_events.trigger_screenshot(),
                        tray::TRAY_MENU_SETTINGS => ctrl_for_events.open_settings(),
                        tray::TRAY_MENU_EXIT => {
                            let _ = slint::quit_event_loop();
                        }
                        _ => {}
                    },
                    tray::SystemMessage::ShutdownRequested => {
                        let _ = slint::quit_event_loop();
                    }
                    tray::SystemMessage::ExplorerRestarted => {
                        #[cfg(windows)]
                        tray::add_tray_icon(notification_hwnd);
                    }
                }
            }

            while let Ok(cmd) = ipc_rx.try_recv() {
                match cmd.as_str() {
                    "OPEN_SETTINGS" => ctrl_for_events.open_settings(),
                    "SHUTDOWN_FOR_UPDATE" => {
                        info!("Received shutdown for update IPC. Quitting event loop.");
                        let _ = slint::quit_event_loop();
                    }
                    _ => {}
                }
            }
        },
    );

    // 10. Run Slint event loop
    info!("Running Slint event loop. System ready.");
    slint::run_event_loop()?;

    // 11. Cleanup on graceful exit
    info!("Cleaning up resources before exit...");
    event_timer.stop();
    controller.shutdown();
    shutdown_flag.store(true, Ordering::Relaxed);

    #[cfg(windows)]
    {
        tray::remove_tray_icon(msg_window.hwnd);
        hotkey::unregister_global_hotkey(msg_window.hwnd, hotkey::HOTKEY_ID_SCREENSHOT);
        hotkey::unregister_global_hotkey(msg_window.hwnd, hotkey::HOTKEY_ID_COLOR);
        hotkey::unregister_global_hotkey(msg_window.hwnd, hotkey::HOTKEY_ID_LONGSHOT);
    }

    info!("Snipe terminated cleanly.");
    Ok(())
}
