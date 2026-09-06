use ai_client::{OpenAiVisionClient, VisionProvider};
use capture_core::{CaptureService, CaptureTarget};
use capture_windows::WindowsCaptureService;
use config::AppConfig;
use domain::{DesktopPxPoint, DesktopPxRect, Frame, ImagePxRect};
use secure_storage_windows::{CredentialStorage, DEFAULT_TARGET_NAME};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;
use tracing::{error, info};
use ui_slint::{AiResultWindow, OverlayWindow, SettingsWindow};

use crate::pin_manager::PinManager;

pub struct AppController {
    capture_service: Arc<WindowsCaptureService>,
    pin_manager: PinManager,
    overlay_window: Rc<OverlayWindow>,
    current_frame: Rc<RefCell<Option<Frame>>>,
    selection_start: Rc<RefCell<Option<(i32, i32)>>>,
    settings_window: Rc<RefCell<Option<SettingsWindow>>>,
    ai_window: Rc<RefCell<Option<AiResultWindow>>>,
}

impl AppController {
    pub fn new() -> Self {
        let capture_service = Arc::new(WindowsCaptureService::new());
        let pin_manager = PinManager::new();
        let overlay_window = Rc::new(OverlayWindow::new().unwrap());

        let controller = Self {
            capture_service,
            pin_manager,
            overlay_window,
            current_frame: Rc::new(RefCell::new(None)),
            selection_start: Rc::new(RefCell::new(None)),
            settings_window: Rc::new(RefCell::new(None)),
            ai_window: Rc::new(RefCell::new(None)),
        };

        controller.setup_overlay_callbacks();
        controller
    }

    fn setup_overlay_callbacks(&self) {
        let overlay = self.overlay_window.clone();
        let start_pos = self.selection_start.clone();
        let frame_ref = self.current_frame.clone();

        // Pointer Down
        let overlay_clone = overlay.clone();
        let start_pos_down = start_pos.clone();
        overlay.on_pointer_down(move |x, y| {
            *start_pos_down.borrow_mut() = Some((x, y));
            overlay_clone.set_has_selection(true);
            overlay_clone.set_selection_x(x);
            overlay_clone.set_selection_y(y);
            overlay_clone.set_selection_w(1);
            overlay_clone.set_selection_h(1);
        });

        // Pointer Move
        let overlay_clone = overlay.clone();
        let start_pos_move = start_pos.clone();
        let frame_move = frame_ref.clone();
        overlay.on_pointer_move(move |x, y| {
            overlay_clone.set_mouse_x(x);
            overlay_clone.set_mouse_y(y);

            // Update real-time pixel color under mouse
            if let Some(ref frame) = *frame_move.borrow() {
                if let Some(color) = frame.pixel_at(x as u32, y as u32) {
                    overlay_clone.set_color_hex(color.to_hex_rgb().into());
                    overlay_clone.set_color_rgb(color.to_rgb_str().into());
                }
            }

            if let Some((sx, sy)) = *start_pos_move.borrow() {
                let rx = sx.min(x);
                let ry = sy.min(y);
                let rw = (sx - x).abs().max(1);
                let rh = (sy - y).abs().max(1);

                overlay_clone.set_selection_x(rx);
                overlay_clone.set_selection_y(ry);
                overlay_clone.set_selection_w(rw);
                overlay_clone.set_selection_h(rh);
                overlay_clone.set_dimension_text(format!("{} × {}", rw, rh).into());
            }
        });

        // Pointer Up
        let start_pos_up = start_pos.clone();
        overlay.on_pointer_up(move |_x, _y| {
            *start_pos_up.borrow_mut() = None;
        });

        // Action: Cancel
        let overlay_clone = overlay.clone();
        overlay.on_action_cancel(move || {
            let _ = overlay_clone.hide();
        });

        // Action: Copy
        let overlay_clone = overlay.clone();
        let frame_copy = frame_ref.clone();
        overlay.on_action_copy(move || {
            if let Some(cropped) = Self::get_selected_subframe(&overlay_clone, &frame_copy) {
                let _ = imaging::copy_frame_to_clipboard(&cropped);
            }
            let _ = overlay_clone.hide();
        });

        // Action: Save
        let overlay_clone = overlay.clone();
        let frame_save = frame_ref.clone();
        overlay.on_action_save(move || {
            if let Some(cropped) = Self::get_selected_subframe(&overlay_clone, &frame_save) {
                let cfg = AppConfig::load();
                let dir = std::path::PathBuf::from(&cfg.output.default_save_dir);
                let _ = imaging::save_frame_atomic(
                    &cropped,
                    &dir,
                    &cfg.output.filename_template,
                    &cfg.output.default_format,
                    cfg.output.jpeg_quality,
                );
            }
            let _ = overlay_clone.hide();
        });

        // Action: Pin
        let overlay_clone = overlay.clone();
        let frame_pin = frame_ref.clone();
        let pin_mgr = self.pin_manager.clone();
        overlay.on_action_pin(move || {
            if let Some(cropped) = Self::get_selected_subframe(&overlay_clone, &frame_pin) {
                pin_mgr.spawn_pin(cropped);
            }
            let _ = overlay_clone.hide();
        });

        // Action: OCR
        let overlay_clone = overlay.clone();
        let frame_ocr = frame_ref.clone();
        let ai_win = self.ai_window.clone();
        overlay.on_action_ocr(move || {
            if let Some(cropped) = Self::get_selected_subframe(&overlay_clone, &frame_ocr) {
                Self::perform_ai_task(cropped, false, ai_win.clone());
            }
            let _ = overlay_clone.hide();
        });

        // Action: Translate
        let overlay_clone = overlay.clone();
        let frame_trans = frame_ref.clone();
        let ai_win_trans = self.ai_window.clone();
        overlay.on_action_translate(move || {
            if let Some(cropped) = Self::get_selected_subframe(&overlay_clone, &frame_trans) {
                Self::perform_ai_task(cropped, true, ai_win_trans.clone());
            }
            let _ = overlay_clone.hide();
        });
    }

    fn get_selected_subframe(
        overlay: &OverlayWindow,
        frame_ref: &Rc<RefCell<Option<Frame>>>,
    ) -> Option<Frame> {
        let frame_guard = frame_ref.borrow();
        let frame = frame_guard.as_ref()?;

        let x = overlay.get_selection_x().max(0) as u32;
        let y = overlay.get_selection_y().max(0) as u32;
        let w = overlay.get_selection_w().max(1) as u32;
        let h = overlay.get_selection_h().max(1) as u32;

        frame.crop(ImagePxRect::new(x, y, w, h)).ok()
    }

    pub fn trigger_screenshot(&self) {
        let capture_service = self.capture_service.clone();
        let overlay = self.overlay_window.clone();
        let frame_slot = self.current_frame.clone();

        slint::spawn_local(async move {
            info!("Triggering screen capture...");
            match capture_service.capture(CaptureTarget::VirtualDesktop).await {
                Ok(frame) => {
                    *frame_slot.borrow_mut() = Some(frame);
                    overlay.set_has_selection(false);
                    overlay.set_selection_w(0);
                    overlay.set_selection_h(0);
                    let _ = overlay.show();
                    info!("Showing screenshot overlay");
                }
                Err(e) => {
                    error!("Screenshot capture failed: {:?}", e);
                }
            }
        })
        .unwrap();
    }

    pub fn trigger_color_picker(&self) {
        let capture_service = self.capture_service.clone();
        let overlay = self.overlay_window.clone();
        let frame_slot = self.current_frame.clone();

        slint::spawn_local(async move {
            info!("Triggering color picker...");
            if let Ok(frame) = capture_service.capture(CaptureTarget::VirtualDesktop).await {
                *frame_slot.borrow_mut() = Some(frame);
                overlay.set_has_selection(false);
                let _ = overlay.show();
            }
        })
        .unwrap();
    }

    pub fn open_settings(&self) {
        let mut settings_guard = self.settings_window.borrow_mut();
        if settings_guard.is_none() {
            let win = SettingsWindow::new().unwrap();
            let cfg = AppConfig::load();

            win.set_hotkey_screenshot(cfg.hotkeys.screenshot.into());
            win.set_hotkey_color(cfg.hotkeys.color_picker.into());
            win.set_hotkey_longshot(cfg.hotkeys.longshot.into());
            win.set_auto_start(cfg.ui.auto_start);
            win.set_ai_base_url(cfg.ai.base_url.into());
            win.set_ai_model(cfg.ai.model.into());

            let current_key = CredentialStorage::read_secret(DEFAULT_TARGET_NAME).unwrap_or_default();
            win.set_ai_key_masked(CredentialStorage::mask_key(&current_key).into());

            // Save settings callback
            let win_clone = win.as_weak();
            win.on_save_settings(move || {
                if let Some(w) = win_clone.upgrade() {
                    let mut current_cfg = AppConfig::load();
                    current_cfg.hotkeys.screenshot = w.get_hotkey_screenshot().to_string();
                    current_cfg.hotkeys.color_picker = w.get_hotkey_color().to_string();
                    current_cfg.hotkeys.longshot = w.get_hotkey_longshot().to_string();
                    current_cfg.ui.auto_start = w.get_auto_start();
                    current_cfg.ai.base_url = w.get_ai_base_url().to_string();
                    current_cfg.ai.model = w.get_ai_model().to_string();
                    let _ = current_cfg.save();

                    let new_key = w.get_ai_new_key().to_string();
                    if !new_key.trim().is_empty() {
                        let _ = CredentialStorage::store_secret(DEFAULT_TARGET_NAME, &new_key);
                        w.set_ai_key_masked(CredentialStorage::mask_key(&new_key).into());
                        w.set_ai_new_key("".into());
                    }
                    w.set_ai_test_status("配置已成功保存！".into());
                }
            });

            // Test AI connection callback
            let win_test = win.as_weak();
            win.on_test_ai_connection(move || {
                if let Some(w) = win_test.upgrade() {
                    w.set_ai_test_status("正在测试 API 连接...".into());
                    let base_url = w.get_ai_base_url().to_string();
                    let model = w.get_ai_model().to_string();
                    let input_key = w.get_ai_new_key().to_string();
                    let key = if !input_key.trim().is_empty() {
                        input_key
                    } else {
                        CredentialStorage::read_secret(DEFAULT_TARGET_NAME).unwrap_or_default()
                    };

                    let w_async = w.as_weak();
                    slint::spawn_local(async move {
                        let client = OpenAiVisionClient::new(base_url, key, model, 15);
                        let dummy_png = vec![
                            0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D,
                            0x49, 0x48, 0x44, 0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01,
                            0x08, 0x06, 0x00, 0x00, 0x00, 0x1F, 0x15, 0xC4, 0x89, 0x00, 0x00, 0x00,
                            0x0A, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9C, 0x63, 0x00, 0x01, 0x00, 0x00,
                            0x05, 0x00, 0x01, 0x0D, 0x0A, 0x2D, 0xB4, 0x00, 0x00, 0x00, 0x00, 0x49,
                            0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
                        ];

                        let res = client
                            .recognize(&dummy_png, "png", CancellationToken::new())
                            .await;

                        if let Some(w) = w_async.upgrade() {
                            match res {
                                Ok(_) => w.set_ai_test_status("连接测试成功！".into()),
                                Err(e) => w.set_ai_test_status(format!("测试失败: {}", e).into()),
                            }
                        }
                    })
                    .unwrap();
                }
            });

            *settings_guard = Some(win);
        }

        if let Some(ref win) = *settings_guard {
            let _ = win.show();
        }
    }

    fn perform_ai_task(
        frame: Frame,
        is_translate: bool,
        ai_win_slot: Rc<RefCell<Option<AiResultWindow>>>,
    ) {
        let mut guard = ai_win_slot.borrow_mut();
        if guard.is_none() {
            let w = AiResultWindow::new().unwrap();
            let w_copy = w.as_weak();
            w.on_copy_clicked(move || {
                if let Some(win) = w_copy.upgrade() {
                    let _ = imaging::copy_text_to_clipboard(&win.get_content_text());
                    win.set_status_text("文本已复制到剪贴板".into());
                }
            });

            let w_close = w.as_weak();
            w.on_close_clicked(move || {
                if let Some(win) = w_close.upgrade() {
                    let _ = win.hide();
                }
            });

            *guard = Some(w);
        }

        let win = guard.as_ref().unwrap();
        let cfg = AppConfig::load();
        let api_key = CredentialStorage::read_secret(DEFAULT_TARGET_NAME).unwrap_or_default();

        win.set_title_text(if is_translate { "截图翻译结果".into() } else { "OCR 识别结果".into() });
        win.set_model_name(cfg.ai.model.clone().into());
        win.set_is_loading(true);
        win.set_status_text("正在调用多模态模型识别...".into());
        let _ = win.show();

        let win_weak = win.as_weak();
        let target_lang = cfg.ai.target_language.clone();

        slint::spawn_local(async move {
            let client = OpenAiVisionClient::new(
                cfg.ai.base_url,
                api_key,
                cfg.ai.model,
                cfg.ai.timeout_seconds,
            );

            let png_bytes = match imaging::encode_frame(&frame, "png", 90) {
                Ok(b) => b,
                Err(e) => {
                    if let Some(w) = win_weak.upgrade() {
                        w.set_is_loading(false);
                        w.set_status_text(format!("编码失败: {}", e).into());
                    }
                    return;
                }
            };

            let cancel = CancellationToken::new();

            if is_translate {
                match client.translate_image(&png_bytes, &target_lang, cancel).await {
                    Ok(res) => {
                        if let Some(w) = win_weak.upgrade() {
                            w.set_is_loading(false);
                            w.set_content_text(res.translated_text.into());
                            w.set_status_text("翻译完成".into());
                        }
                    }
                    Err(e) => {
                        if let Some(w) = win_weak.upgrade() {
                            w.set_is_loading(false);
                            w.set_status_text(format!("翻译出错: {}", e).into());
                        }
                    }
                }
            } else {
                match client.recognize(&png_bytes, "png", cancel).await {
                    Ok(res) => {
                        if let Some(w) = win_weak.upgrade() {
                            w.set_is_loading(false);
                            w.set_content_text(res.text.into());
                            w.set_status_text("OCR 识别完成".into());
                        }
                    }
                    Err(e) => {
                        if let Some(w) = win_weak.upgrade() {
                            w.set_is_loading(false);
                            w.set_status_text(format!("OCR 出错: {}", e).into());
                        }
                    }
                }
            }
        })
        .unwrap();
    }
}
