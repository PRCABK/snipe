#![windows_subsystem = "windows"]

mod controller;
mod hotkey;
mod pin_manager;
mod single_instance;
mod tray;

use config::AppConfig;
use domain::HotkeyAction;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::mpsc::unbounded_channel;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use ui_slint::{ComponentHandle, UpdateWindow};

fn wait_for_primary_exit(timeout: std::time::Duration) -> bool {
    let deadline = std::time::Instant::now() + timeout;
    while std::time::Instant::now() < deadline {
        if single_instance::request_from_primary_with_timeout(
            single_instance::IpcCommand::Ping,
            std::time::Duration::from_millis(250),
        )
        .is_none()
        {
            return true;
        }
        std::thread::sleep(std::time::Duration::from_millis(200));
    }
    false
}

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
        info!("Requesting graceful shutdown from primary instance...");
        match single_instance::request_from_primary(single_instance::IpcCommand::ShutdownForUpdate)
        {
            Some(single_instance::IpcResponse::Ready) => {
                if wait_for_primary_exit(std::time::Duration::from_secs(15)) {
                    return Ok(());
                }
                anyhow::bail!("Primary Snipe instance did not exit within 15 seconds")
            }
            Some(response) => anyhow::bail!(
                "Primary instance declined update shutdown: {}",
                response.as_wire()
            ),
            None => anyhow::bail!("Could not contact the primary Snipe instance"),
        }
    }

    // 3. Single instance enforcement
    let _instance_guard = match single_instance::try_acquire_instance() {
        Some(guard) => guard,
        None => {
            info!("Existing instance detected. Requesting settings window...");
            let _ =
                single_instance::request_from_primary(single_instance::IpcCommand::OpenSettings);
            return Ok(());
        }
    };

    // 4. Setup Tokio runtime for background tasks
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;
    let _enter = rt.enter();

    // 5. Setup channels & cancellation
    let (ipc_tx, mut ipc_rx) = unbounded_channel::<single_instance::IpcRequest>();
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
    let update_window = Rc::new(RefCell::new(None::<UpdateWindow>));
    let pending_update_reply = Rc::new(RefCell::new(None));

    // 9. Drain native/IPC events from the Slint thread. Moving AppController
    // into tokio::spawn would require its Slint windows to be Send.
    #[cfg(windows)]
    let notification_hwnd = msg_window.hwnd;
    let ctrl_for_events = controller.clone();
    let update_window_for_events = update_window.clone();
    let pending_update_reply_for_events = pending_update_reply.clone();
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
                        HotkeyAction::Longshot => ctrl_for_events.trigger_longshot(),
                        HotkeyAction::Pin => ctrl_for_events.trigger_screenshot(),
                    },
                    tray::SystemMessage::TrayAction(menu_id) => match menu_id {
                        tray::TRAY_MENU_SCREENSHOT => ctrl_for_events.trigger_screenshot(),
                        tray::TRAY_MENU_COLOR => ctrl_for_events.trigger_color_picker(),
                        tray::TRAY_MENU_LONGSHOT => ctrl_for_events.trigger_longshot(),
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

            while let Ok(request) = ipc_rx.try_recv() {
                let response = match request.command {
                    single_instance::IpcCommand::OpenSettings => {
                        ctrl_for_events.open_settings();
                        single_instance::IpcResponse::Ready
                    }
                    single_instance::IpcCommand::Ping => single_instance::IpcResponse::Ready,
                    single_instance::IpcCommand::ShutdownForUpdate => {
                        if ctrl_for_events.has_active_task()
                            || pending_update_reply_for_events.borrow().is_some()
                        {
                            single_instance::IpcResponse::Busy
                        } else {
                            let window = if let Some(window) =
                                update_window_for_events.borrow().as_ref()
                            {
                                window.clone()
                            } else {
                                let window =
                                    UpdateWindow::new().expect("create update confirmation window");
                                let pending_confirm = pending_update_reply_for_events.clone();
                                let window_confirm = window.as_weak();
                                window.on_action_confirm(move || {
                                    if let Some(reply) = pending_confirm.borrow_mut().take() {
                                        let _ = reply.send(single_instance::IpcResponse::Ready);
                                    }
                                    if let Some(window) = window_confirm.upgrade() {
                                        let _ = window.hide();
                                    }
                                    let _ = slint::quit_event_loop();
                                });
                                let pending_cancel = pending_update_reply_for_events.clone();
                                let window_cancel = window.as_weak();
                                window.on_action_cancel(move || {
                                    if let Some(reply) = pending_cancel.borrow_mut().take() {
                                        let _ = reply.send(single_instance::IpcResponse::Cancelled);
                                    }
                                    if let Some(window) = window_cancel.upgrade() {
                                        let _ = window.hide();
                                    }
                                });
                                *update_window_for_events.borrow_mut() = Some(window.clone());
                                window
                            };
                            *pending_update_reply_for_events.borrow_mut() = Some(request.response);
                            let _ = window.show();
                            continue;
                        }
                    }
                };
                let _ = request.response.send(response);
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
