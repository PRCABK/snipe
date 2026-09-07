use ai_client::{OpenAiVisionClient, VisionProvider};
use annotation::{
    composite_annotations, AnnotationItem, AnnotationKind, ArrowShape, CommandStack, MosaicShape,
    PathShape, RectShape, StepShape,
};
use capture_core::{CaptureService, CaptureTarget};
use capture_windows::WindowsCaptureService;
use config::AppConfig;
use domain::{
    desktop_rect_to_image_rect, desktop_to_logical, logical_to_desktop, ColorRgba, DesktopPxPoint,
    DesktopPxRect, Frame, ImagePxRect, LogicalPoint,
};
use secure_storage_windows::{CredentialStorage, DEFAULT_TARGET_NAME};
use slint::{ComponentHandle, PhysicalPosition, PhysicalSize};
use std::cell::RefCell;
use std::rc::{Rc, Weak as RcWeak};
use std::sync::Arc;
use tokio_util::sync::CancellationToken;
use tracing::{error, info};
use ui_slint::{
    frame_to_slint_image, AiResultWindow, EditorWindow, LongshotWindow, OverlayWindow,
    SettingsWindow,
};

use crate::pin_manager::PinManager;

const LONGSHOT_MAX_FRAMES: usize = 300;
const LONGSHOT_MAX_HEIGHT: u32 = 30_000;
const LONGSHOT_MAX_DURATION: std::time::Duration = std::time::Duration::from_secs(5 * 60);

struct OverlaySurface {
    window: Rc<OverlayWindow>,
    monitor_bounds: DesktopPxRect,
    scale_factor: f32,
}

struct LongshotState {
    target_rect: DesktopPxRect,
    session: longshot::LongshotSession,
    result: Option<Frame>,
}

struct EditorState {
    base_frame: Frame,
    commands: CommandStack,
    active_tool: String,
    active_color: ColorRgba,
    pointer_start: Option<(f32, f32)>,
    freehand_points: Vec<(f32, f32)>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum CaptureMode {
    Screenshot,
    LongshotSelection,
}

pub struct AppController {
    capture_service: Arc<WindowsCaptureService>,
    pin_manager: PinManager,
    overlays: Rc<RefCell<Vec<OverlaySurface>>>,
    current_frame: Rc<RefCell<Option<Frame>>>,
    frame_origin: Rc<RefCell<DesktopPxPoint>>,
    selection_start: Rc<RefCell<Option<DesktopPxPoint>>>,
    selection_rect: Rc<RefCell<Option<DesktopPxRect>>>,
    capture_mode: Rc<RefCell<CaptureMode>>,
    longshot_window: Rc<RefCell<Option<LongshotWindow>>>,
    longshot_state: Rc<RefCell<Option<LongshotState>>>,
    editor_window: Rc<RefCell<Option<EditorWindow>>>,
    editor_state: Rc<RefCell<Option<EditorState>>>,
    settings_window: Rc<RefCell<Option<SettingsWindow>>>,
    ai_window: Rc<RefCell<Option<AiResultWindow>>>,
}

impl AppController {
    pub fn new() -> Self {
        let capture_service = Arc::new(WindowsCaptureService::new());
        let pin_manager = PinManager::new();

        Self {
            capture_service,
            pin_manager,
            overlays: Rc::new(RefCell::new(Vec::new())),
            current_frame: Rc::new(RefCell::new(None)),
            frame_origin: Rc::new(RefCell::new(DesktopPxPoint::default())),
            selection_start: Rc::new(RefCell::new(None)),
            selection_rect: Rc::new(RefCell::new(None)),
            capture_mode: Rc::new(RefCell::new(CaptureMode::Screenshot)),
            longshot_window: Rc::new(RefCell::new(None)),
            longshot_state: Rc::new(RefCell::new(None)),
            editor_window: Rc::new(RefCell::new(None)),
            editor_state: Rc::new(RefCell::new(None)),
            settings_window: Rc::new(RefCell::new(None)),
            ai_window: Rc::new(RefCell::new(None)),
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn setup_overlay_callbacks(
        overlay: Rc<OverlayWindow>,
        monitor_bounds: DesktopPxRect,
        scale_factor: f32,
        overlays: RcWeak<RefCell<Vec<OverlaySurface>>>,
        frame_ref: Rc<RefCell<Option<Frame>>>,
        frame_origin: Rc<RefCell<DesktopPxPoint>>,
        selection_start: Rc<RefCell<Option<DesktopPxPoint>>>,
        selection_rect: Rc<RefCell<Option<DesktopPxRect>>>,
        capture_service: Arc<WindowsCaptureService>,
        capture_mode: Rc<RefCell<CaptureMode>>,
        longshot_window: Rc<RefCell<Option<LongshotWindow>>>,
        longshot_state: Rc<RefCell<Option<LongshotState>>>,
        editor_window: Rc<RefCell<Option<EditorWindow>>>,
        editor_state: Rc<RefCell<Option<EditorState>>>,
        pin_manager: PinManager,
        ai_window: Rc<RefCell<Option<AiResultWindow>>>,
    ) {
        let monitor_origin = DesktopPxPoint::new(monitor_bounds.x, monitor_bounds.y);

        let overlay_down = overlay.as_weak();
        let selection_start_down = selection_start.clone();
        let selection_rect_down = selection_rect.clone();
        let overlays_down = overlays.clone();
        overlay.on_pointer_down(move |x, y| {
            let desktop_point =
                logical_to_desktop(LogicalPoint::new(x, y), scale_factor, monitor_origin);
            *selection_start_down.borrow_mut() = Some(desktop_point);
            let selection = DesktopPxRect::new(desktop_point.x, desktop_point.y, 1, 1);
            *selection_rect_down.borrow_mut() = Some(selection);
            Self::update_overlay_selection(&overlays_down, selection);
            if let Some(overlay) = overlay_down.upgrade() {
                overlay.set_has_selection(true);
            }
        });

        let overlay_move = overlay.as_weak();
        let selection_start_move = selection_start.clone();
        let selection_rect_move = selection_rect.clone();
        let overlays_move = overlays.clone();
        let frame_move = frame_ref.clone();
        let frame_origin_move = frame_origin.clone();
        overlay.on_pointer_move(move |x, y| {
            let desktop_point =
                logical_to_desktop(LogicalPoint::new(x, y), scale_factor, monitor_origin);
            let monitor_x = desktop_point.x - monitor_bounds.x;
            let monitor_y = desktop_point.y - monitor_bounds.y;
            if let Some(overlay) = overlay_move.upgrade() {
                overlay.set_mouse_x(monitor_x);
                overlay.set_mouse_y(monitor_y);

                if let Some(frame) = frame_move.borrow().as_ref() {
                    let point_rect = DesktopPxRect::new(desktop_point.x, desktop_point.y, 1, 1);
                    if let Ok(image_rect) = desktop_rect_to_image_rect(
                        point_rect,
                        *frame_origin_move.borrow(),
                        frame.width,
                        frame.height,
                    ) {
                        if let Some(color) = frame.pixel_at(image_rect.x, image_rect.y) {
                            overlay.set_color_hex(color.to_hex_rgb().into());
                            overlay.set_color_rgb(color.to_rgb_str().into());
                            overlay.set_preview_color(slint::Color::from_argb_u8(
                                color.a, color.r, color.g, color.b,
                            ));
                        }
                    }
                }
            }

            if let Some(start) = *selection_start_move.borrow() {
                let selection = Self::selection_from_points(start, desktop_point);
                *selection_rect_move.borrow_mut() = Some(selection);
                Self::update_overlay_selection(&overlays_move, selection);
            }
        });

        let selection_start_up = selection_start.clone();
        let selection_rect_up = selection_rect.clone();
        let overlays_up = overlays.clone();
        let frame_up = frame_ref.clone();
        let frame_origin_up = frame_origin.clone();
        let capture_mode_up = capture_mode.clone();
        let capture_service_up = capture_service.clone();
        let longshot_window_up = longshot_window.clone();
        let longshot_state_up = longshot_state.clone();
        overlay.on_pointer_up(move |_x, _y| {
            *selection_start_up.borrow_mut() = None;
            if *capture_mode_up.borrow() == CaptureMode::LongshotSelection {
                let Some(selection) = *selection_rect_up.borrow() else {
                    return;
                };
                Self::begin_longshot_session(
                    selection,
                    &overlays_up,
                    &frame_up,
                    &frame_origin_up,
                    capture_service_up.clone(),
                    longshot_window_up.clone(),
                    longshot_state_up.clone(),
                );
                *selection_rect_up.borrow_mut() = None;
                *capture_mode_up.borrow_mut() = CaptureMode::Screenshot;
            }
        });

        let overlays_cancel = overlays.clone();
        let frame_cancel = frame_ref.clone();
        let selection_start_cancel = selection_start.clone();
        let selection_rect_cancel = selection_rect.clone();
        overlay.on_action_cancel(move || {
            Self::clear_capture_session_shared(
                &overlays_cancel,
                &frame_cancel,
                &selection_start_cancel,
                &selection_rect_cancel,
            );
        });

        let overlays_copy = overlays.clone();
        let frame_copy = frame_ref.clone();
        let frame_origin_copy = frame_origin.clone();
        let selection_copy = selection_rect.clone();
        let selection_start_copy = selection_start.clone();
        overlay.on_action_copy(move || {
            if let Some(cropped) =
                Self::get_selected_subframe(&frame_copy, &frame_origin_copy, &selection_copy)
            {
                let _ = imaging::copy_frame_to_clipboard(&cropped);
            }
            Self::clear_capture_session_shared(
                &overlays_copy,
                &frame_copy,
                &selection_start_copy,
                &selection_copy,
            );
        });

        let overlays_save = overlays.clone();
        let frame_save = frame_ref.clone();
        let frame_origin_save = frame_origin.clone();
        let selection_save = selection_rect.clone();
        let selection_start_save = selection_start.clone();
        overlay.on_action_save(move || {
            if let Some(cropped) =
                Self::get_selected_subframe(&frame_save, &frame_origin_save, &selection_save)
            {
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
            Self::clear_capture_session_shared(
                &overlays_save,
                &frame_save,
                &selection_start_save,
                &selection_save,
            );
        });

        let overlays_pin = overlays.clone();
        let frame_pin = frame_ref.clone();
        let frame_origin_pin = frame_origin.clone();
        let selection_pin = selection_rect.clone();
        let selection_start_pin = selection_start.clone();
        let pin_manager_pin = pin_manager.clone();
        overlay.on_action_pin(move || {
            if let Some(cropped) =
                Self::get_selected_subframe(&frame_pin, &frame_origin_pin, &selection_pin)
            {
                pin_manager_pin.spawn_pin(cropped);
            }
            Self::clear_capture_session_shared(
                &overlays_pin,
                &frame_pin,
                &selection_start_pin,
                &selection_pin,
            );
        });

        let overlays_annotate = overlays.clone();
        let frame_annotate = frame_ref.clone();
        let frame_origin_annotate = frame_origin.clone();
        let selection_annotate = selection_rect.clone();
        let selection_start_annotate = selection_start.clone();
        let editor_window_annotate = editor_window.clone();
        let editor_state_annotate = editor_state.clone();
        let pin_manager_annotate = pin_manager.clone();
        overlay.on_action_annotate(move || {
            if let Some(cropped) = Self::get_selected_subframe(
                &frame_annotate,
                &frame_origin_annotate,
                &selection_annotate,
            ) {
                Self::open_editor(
                    cropped,
                    editor_window_annotate.clone(),
                    editor_state_annotate.clone(),
                    pin_manager_annotate.clone(),
                );
            }
            Self::clear_capture_session_shared(
                &overlays_annotate,
                &frame_annotate,
                &selection_start_annotate,
                &selection_annotate,
            );
        });

        let overlays_ocr = overlays.clone();
        let frame_ocr = frame_ref.clone();
        let frame_origin_ocr = frame_origin.clone();
        let selection_ocr = selection_rect.clone();
        let selection_start_ocr = selection_start.clone();
        let ai_window_ocr = ai_window.clone();
        overlay.on_action_ocr(move || {
            if let Some(cropped) =
                Self::get_selected_subframe(&frame_ocr, &frame_origin_ocr, &selection_ocr)
            {
                Self::perform_ai_task(cropped, false, ai_window_ocr.clone());
            }
            Self::clear_capture_session_shared(
                &overlays_ocr,
                &frame_ocr,
                &selection_start_ocr,
                &selection_ocr,
            );
        });

        let overlays_translate = overlays.clone();
        let frame_translate = frame_ref.clone();
        let frame_origin_translate = frame_origin.clone();
        let selection_translate = selection_rect.clone();
        let selection_start_translate = selection_start.clone();
        overlay.on_action_translate(move || {
            if let Some(cropped) = Self::get_selected_subframe(
                &frame_translate,
                &frame_origin_translate,
                &selection_translate,
            ) {
                Self::perform_ai_task(cropped, true, ai_window.clone());
            }
            Self::clear_capture_session_shared(
                &overlays_translate,
                &frame_translate,
                &selection_start_translate,
                &selection_translate,
            );
        });
    }

    fn selection_from_points(start: DesktopPxPoint, end: DesktopPxPoint) -> DesktopPxRect {
        let mut selection = DesktopPxRect::normalize(start, end);
        selection.width = selection.width.max(1);
        selection.height = selection.height.max(1);
        selection
    }

    fn update_overlay_selection(
        overlays: &RcWeak<RefCell<Vec<OverlaySurface>>>,
        selection: DesktopPxRect,
    ) {
        let Some(overlays) = overlays.upgrade() else {
            return;
        };
        for surface in overlays.borrow().iter() {
            let monitor_rect = surface.monitor_bounds;
            let x = selection.x.max(monitor_rect.x);
            let y = selection.y.max(monitor_rect.y);
            let right = selection.right().min(monitor_rect.right());
            let bottom = selection.bottom().min(monitor_rect.bottom());

            if right <= x || bottom <= y {
                surface.window.set_has_selection(false);
                continue;
            }

            let logical_top_left = desktop_to_logical(
                DesktopPxPoint::new(x, y),
                surface.scale_factor,
                DesktopPxPoint::new(surface.monitor_bounds.x, surface.monitor_bounds.y),
            );
            let logical_bottom_right = desktop_to_logical(
                DesktopPxPoint::new(right, bottom),
                surface.scale_factor,
                DesktopPxPoint::new(surface.monitor_bounds.x, surface.monitor_bounds.y),
            );
            surface.window.set_has_selection(true);
            surface
                .window
                .set_selection_x(logical_top_left.x.round() as i32);
            surface
                .window
                .set_selection_y(logical_top_left.y.round() as i32);
            surface.window.set_selection_w(
                (logical_bottom_right.x - logical_top_left.x)
                    .round()
                    .max(1.0) as i32,
            );
            surface.window.set_selection_h(
                (logical_bottom_right.y - logical_top_left.y)
                    .round()
                    .max(1.0) as i32,
            );
            surface
                .window
                .set_dimension_text(format!("{} × {}", selection.width, selection.height).into());
        }
    }

    fn get_selected_subframe(
        frame_ref: &Rc<RefCell<Option<Frame>>>,
        frame_origin: &Rc<RefCell<DesktopPxPoint>>,
        selection_rect: &Rc<RefCell<Option<DesktopPxRect>>>,
    ) -> Option<Frame> {
        let selection = (*selection_rect.borrow())?;
        if selection.width == 0 || selection.height == 0 {
            return None;
        }

        let frame_guard = frame_ref.borrow();
        let frame = frame_guard.as_ref()?;
        let image_rect = desktop_rect_to_image_rect(
            selection,
            *frame_origin.borrow(),
            frame.width,
            frame.height,
        )
        .ok()?;
        frame.crop(image_rect).ok()
    }

    fn clear_capture_session(&self) {
        Self::clear_capture_session_shared(
            &Rc::downgrade(&self.overlays),
            &self.current_frame,
            &self.selection_start,
            &self.selection_rect,
        );
    }

    fn clear_capture_session_shared(
        overlays: &RcWeak<RefCell<Vec<OverlaySurface>>>,
        frame_ref: &Rc<RefCell<Option<Frame>>>,
        selection_start: &Rc<RefCell<Option<DesktopPxPoint>>>,
        selection_rect: &Rc<RefCell<Option<DesktopPxRect>>>,
    ) {
        if let Some(overlays) = overlays.upgrade() {
            for surface in overlays.borrow().iter() {
                surface.window.set_capture_image(slint::Image::default());
                surface.window.set_has_selection(false);
                surface.window.set_selection_w(0);
                surface.window.set_selection_h(0);
                let _ = surface.window.hide();
            }
        }
        *frame_ref.borrow_mut() = None;
        *selection_start.borrow_mut() = None;
        *selection_rect.borrow_mut() = None;
    }

    fn begin_longshot_session(
        target_rect: DesktopPxRect,
        overlays: &RcWeak<RefCell<Vec<OverlaySurface>>>,
        frame_ref: &Rc<RefCell<Option<Frame>>>,
        frame_origin: &Rc<RefCell<DesktopPxPoint>>,
        capture_service: Arc<WindowsCaptureService>,
        window_slot: Rc<RefCell<Option<LongshotWindow>>>,
        state_slot: Rc<RefCell<Option<LongshotState>>>,
    ) {
        let Some(first_frame) = Self::get_selected_subframe(
            frame_ref,
            frame_origin,
            &Rc::new(RefCell::new(Some(target_rect))),
        ) else {
            return;
        };
        let mut session = longshot::LongshotSession::new(
            target_rect,
            longshot::StitchOptions {
                max_height: LONGSHOT_MAX_HEIGHT,
                ..Default::default()
            },
        );
        let _ = session.add_frame(first_frame);
        *state_slot.borrow_mut() = Some(LongshotState {
            target_rect,
            session,
            result: None,
        });
        Self::clear_capture_session_shared(
            overlays,
            frame_ref,
            &Rc::new(RefCell::new(None)),
            &Rc::new(RefCell::new(Some(target_rect))),
        );

        if window_slot.borrow().is_none() {
            let window = LongshotWindow::new().expect("create longshot window");

            let state_capture = state_slot.clone();
            let window_capture = window.as_weak();
            let service_capture = capture_service.clone();
            window.on_action_capture_frame(move || {
                let state_capture = state_capture.clone();
                let window_capture = window_capture.clone();
                let service_capture = service_capture.clone();
                slint::spawn_local(async move {
                    if let Some(window) = window_capture.upgrade() {
                        let _ = window.hide();
                    }
                    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                    let target_rect = {
                        let mut state_guard = state_capture.borrow_mut();
                        let Some(state) = state_guard.as_mut() else {
                            return;
                        };
                        if state.session.frame_count() >= LONGSHOT_MAX_FRAMES {
                            if let Some(window) = window_capture.upgrade() {
                                let _ = window.show();
                                window
                                    .set_status_text("已达到 300 帧上限；可结束拼接或取消".into());
                            }
                            return;
                        }
                        if state.session.duration() >= LONGSHOT_MAX_DURATION {
                            if let Some(window) = window_capture.upgrade() {
                                let _ = window.show();
                                window
                                    .set_status_text("已达到 5 分钟时限；可结束拼接或取消".into());
                            }
                            return;
                        }
                        state.target_rect
                    };
                    match service_capture
                        .capture(CaptureTarget::Region(target_rect))
                        .await
                    {
                        Ok(frame) => {
                            let mut state_guard = state_capture.borrow_mut();
                            let Some(state) = state_guard.as_mut() else {
                                return;
                            };
                            if state.session.add_frame(frame) {
                                if let Some(window) = window_capture.upgrade() {
                                    let _ = window.show();
                                    let frames = state.session.frame_count();
                                    window.set_frame_count(frames as i32);
                                    window.set_captured_height(
                                        (frames as u32).saturating_mul(state.target_rect.height)
                                            as i32,
                                    );
                                    window.set_can_finish(frames >= 2);
                                    window.set_status_text(
                                        "已采集当前画面；继续滚动后可再次采集".into(),
                                    );
                                }
                            }
                        }
                        Err(error) => {
                            if let Some(window) = window_capture.upgrade() {
                                let _ = window.show();
                                window.set_status_text(
                                    format!("采帧失败，已保留此前帧: {error}").into(),
                                );
                            }
                        }
                    }
                })
                .expect("schedule longshot capture");
            });

            let state_finish = state_slot.clone();
            let window_finish = window.as_weak();
            window.on_action_finish(move || {
                let mut state_guard = state_finish.borrow_mut();
                let Some(state) = state_guard.as_mut() else {
                    return;
                };
                match state.session.finish() {
                    Ok(result) => {
                        let height = result.height;
                        state.result = Some(result);
                        if let Some(window) = window_finish.upgrade() {
                            window.set_has_result(true);
                            window.set_can_finish(false);
                            window.set_captured_height(height as i32);
                            window.set_status_text("拼接完成；可复制、保存或取消".into());
                        }
                    }
                    Err(error) => {
                        if let Some(recovery) = state.session.frames().last().cloned() {
                            state.result = Some(recovery);
                        }
                        if let Some(window) = window_finish.upgrade() {
                            window.set_has_result(state.result.is_some());
                            window.set_status_text(
                                format!("拼接失败，已保留最后一帧供复制或保存: {error}").into(),
                            );
                        }
                    }
                }
            });

            let state_copy = state_slot.clone();
            window.on_action_copy(move || {
                if let Some(frame) = state_copy
                    .borrow()
                    .as_ref()
                    .and_then(|state| state.result.as_ref())
                {
                    let _ = imaging::copy_frame_to_clipboard(frame);
                }
            });

            let state_save = state_slot.clone();
            window.on_action_save(move || {
                if let Some(frame) = state_save
                    .borrow()
                    .as_ref()
                    .and_then(|state| state.result.as_ref())
                {
                    Self::save_frame_using_config(frame);
                }
            });

            let state_cancel = state_slot.clone();
            let window_cancel = window.as_weak();
            window.on_action_cancel(move || {
                *state_cancel.borrow_mut() = None;
                if let Some(window) = window_cancel.upgrade() {
                    let _ = window.hide();
                }
            });
            *window_slot.borrow_mut() = Some(window);
        }

        if let Some(window) = window_slot.borrow().as_ref() {
            window.set_frame_count(1);
            window.set_captured_height(target_rect.height as i32);
            window.set_can_finish(false);
            window.set_has_result(false);
            window.set_status_text("已采集首帧。滚动内容稳定后点击“采集帧”".into());
            let _ = window.show();
        }
    }

    fn save_frame_using_config(frame: &Frame) {
        let cfg = AppConfig::load();
        let dir = std::path::PathBuf::from(&cfg.output.default_save_dir);
        let _ = imaging::save_frame_atomic(
            frame,
            &dir,
            &cfg.output.filename_template,
            &cfg.output.default_format,
            cfg.output.jpeg_quality,
        );
    }

    fn open_editor(
        frame: Frame,
        window_slot: Rc<RefCell<Option<EditorWindow>>>,
        state_slot: Rc<RefCell<Option<EditorState>>>,
        pin_manager: PinManager,
    ) {
        *state_slot.borrow_mut() = Some(EditorState {
            base_frame: frame,
            commands: CommandStack::new(),
            active_tool: "rect".to_string(),
            active_color: ColorRgba::rgb(255, 59, 48),
            pointer_start: None,
            freehand_points: Vec::new(),
        });

        if window_slot.borrow().is_none() {
            let window = EditorWindow::new().expect("create annotation editor");

            let state_tool = state_slot.clone();
            let window_tool = window.as_weak();
            window.on_select_tool(move |tool| {
                if let Some(state) = state_tool.borrow_mut().as_mut() {
                    state.active_tool = tool.to_string();
                    if let Some(window) = window_tool.upgrade() {
                        window.set_active_tool(tool);
                    }
                }
            });

            let state_color = state_slot.clone();
            let window_color = window.as_weak();
            window.on_select_color(move |color| {
                if let Some(state) = state_color.borrow_mut().as_mut() {
                    state.active_color = Self::color_from_hex(color.as_str());
                    if let Some(window) = window_color.upgrade() {
                        window.set_active_color(color);
                    }
                }
            });

            let state_down = state_slot.clone();
            let window_down = window.as_weak();
            window.on_canvas_pointer_down(move |x, y| {
                if let Some(state) = state_down.borrow_mut().as_mut() {
                    let point = Self::editor_point(&state.base_frame, x, y);
                    state.pointer_start = Some(point);
                    state.freehand_points = vec![point];
                    if state.active_tool == "step" {
                        let item = AnnotationItem::new(AnnotationKind::Step(StepShape {
                            cx: point.0,
                            cy: point.1,
                            number: state.commands.items().len() as u32 + 1,
                            bg_color: state.active_color,
                            text_color: ColorRgba::rgb(255, 255, 255),
                            radius: 16.0,
                        }));
                        state.commands.add_item(item);
                        state.pointer_start = None;
                        Self::refresh_editor(&window_down, state);
                    }
                }
            });

            let state_move = state_slot.clone();
            window.on_canvas_pointer_move(move |x, y| {
                if let Some(state) = state_move.borrow_mut().as_mut() {
                    if state.pointer_start.is_some() && state.active_tool == "pen" {
                        let point = Self::editor_point(&state.base_frame, x, y);
                        if state.freehand_points.last().copied() != Some(point) {
                            state.freehand_points.push(point);
                        }
                    }
                }
            });

            let state_up = state_slot.clone();
            let window_up = window.as_weak();
            window.on_canvas_pointer_up(move |x, y| {
                let mut state_borrow = state_up.borrow_mut();
                let Some(state) = state_borrow.as_mut() else {
                    return;
                };
                let Some(start) = state.pointer_start.take() else {
                    return;
                };
                let end = Self::editor_point(&state.base_frame, x, y);
                let color = state.active_color;
                let item = match state.active_tool.as_str() {
                    "rect" => AnnotationItem::new(AnnotationKind::Rect(RectShape {
                        x: start.0.min(end.0),
                        y: start.1.min(end.1),
                        width: (end.0 - start.0).abs().max(1.0),
                        height: (end.1 - start.1).abs().max(1.0),
                        stroke_color: color,
                        stroke_width: 4.0,
                        fill_color: None,
                    })),
                    "arrow" => AnnotationItem::new(AnnotationKind::Arrow(ArrowShape {
                        x1: start.0,
                        y1: start.1,
                        x2: end.0,
                        y2: end.1,
                        stroke_color: color,
                        stroke_width: 4.0,
                    })),
                    "pen" => AnnotationItem::new(AnnotationKind::Freehand(PathShape {
                        points: std::mem::take(&mut state.freehand_points),
                        stroke_color: color,
                        stroke_width: 4.0,
                        is_highlighter: false,
                    })),
                    "mosaic" => AnnotationItem::new(AnnotationKind::Mosaic(MosaicShape {
                        rect: ImagePxRect::new(
                            start.0.min(end.0).round() as u32,
                            start.1.min(end.1).round() as u32,
                            (end.0 - start.0).abs().round().max(1.0) as u32,
                            (end.1 - start.1).abs().round().max(1.0) as u32,
                        ),
                        block_size: 12,
                    })),
                    _ => return,
                };
                state.commands.add_item(item);
                Self::refresh_editor(&window_up, state);
            });

            let state_undo = state_slot.clone();
            let window_undo = window.as_weak();
            window.on_action_undo(move || {
                if let Some(state) = state_undo.borrow_mut().as_mut() {
                    state.commands.undo();
                    Self::refresh_editor(&window_undo, state);
                }
            });

            let state_redo = state_slot.clone();
            let window_redo = window.as_weak();
            window.on_action_redo(move || {
                if let Some(state) = state_redo.borrow_mut().as_mut() {
                    state.commands.redo();
                    Self::refresh_editor(&window_redo, state);
                }
            });

            let state_finish = state_slot.clone();
            let window_finish = window.as_weak();
            let pin_manager_finish = pin_manager.clone();
            window.on_action_finish(move || {
                let Some(state) = state_finish.borrow_mut().take() else {
                    return;
                };
                let result = composite_annotations(&state.base_frame, state.commands.items());
                let _ = imaging::copy_frame_to_clipboard(&result);
                Self::save_frame_using_config(&result);
                pin_manager_finish.spawn_pin(result);
                if let Some(window) = window_finish.upgrade() {
                    let _ = window.hide();
                }
            });

            let state_cancel = state_slot.clone();
            let window_cancel = window.as_weak();
            window.on_action_cancel(move || {
                *state_cancel.borrow_mut() = None;
                if let Some(window) = window_cancel.upgrade() {
                    let _ = window.hide();
                }
            });
            *window_slot.borrow_mut() = Some(window);
        }

        if let Some(window) = window_slot.borrow().as_ref() {
            if let Some(state) = state_slot.borrow().as_ref() {
                Self::refresh_editor_window(window, state);
            }
            let _ = window.show();
        }
    }

    fn editor_point(frame: &Frame, x: f32, y: f32) -> (f32, f32) {
        (
            x.clamp(0.0, 1.0) * frame.width.saturating_sub(1) as f32,
            y.clamp(0.0, 1.0) * frame.height.saturating_sub(1) as f32,
        )
    }

    fn refresh_editor(window: &slint::Weak<EditorWindow>, state: &EditorState) {
        if let Some(window) = window.upgrade() {
            Self::refresh_editor_window(&window, state);
        }
    }

    fn refresh_editor_window(window: &EditorWindow, state: &EditorState) {
        let preview = composite_annotations(&state.base_frame, state.commands.items());
        window.set_editor_image(frame_to_slint_image(&preview));
        window.set_can_undo(state.commands.can_undo());
        window.set_can_redo(state.commands.can_redo());
    }

    fn color_from_hex(value: &str) -> ColorRgba {
        let value = value.trim_start_matches('#');
        if value.len() == 6 {
            let parse = |range| u8::from_str_radix(&value[range], 16).unwrap_or(0);
            return ColorRgba::rgb(parse(0..2), parse(2..4), parse(4..6));
        }
        ColorRgba::rgb(255, 59, 48)
    }

    pub fn trigger_longshot(&self) {
        if self.has_active_task() {
            info!("Longshot request ignored because another capture task is active");
            return;
        }
        *self.capture_mode.borrow_mut() = CaptureMode::LongshotSelection;
        self.trigger_screenshot();
    }

    pub fn has_active_task(&self) -> bool {
        self.current_frame.borrow().is_some()
            || self.longshot_state.borrow().is_some()
            || self.editor_state.borrow().is_some()
            || self.pin_manager.has_pins()
    }

    pub fn trigger_screenshot(&self) {
        if self.current_frame.borrow().is_some() {
            info!("Screenshot request ignored because a capture session is active");
            return;
        }
        let capture_service = self.capture_service.clone();
        let overlays = self.overlays.clone();
        let frame_slot = self.current_frame.clone();
        let frame_origin = self.frame_origin.clone();
        let selection_start = self.selection_start.clone();
        let selection_rect = self.selection_rect.clone();
        let self_capture_mode = self.capture_mode.clone();
        let longshot_window = self.longshot_window.clone();
        let longshot_state = self.longshot_state.clone();
        let editor_window = self.editor_window.clone();
        let editor_state = self.editor_state.clone();
        let pin_manager = self.pin_manager.clone();
        let ai_window = self.ai_window.clone();

        slint::spawn_local(async move {
            info!("Triggering screen capture...");
            let displays = match capture_service.displays().await {
                Ok(displays) => displays,
                Err(error) => {
                    error!("Display enumeration failed: {:?}", error);
                    return;
                }
            };
            let virtual_desktop = capture_windows::display::get_virtual_desktop_bounds(&displays);
            let frame = match capture_service
                .capture(CaptureTarget::Region(virtual_desktop))
                .await
            {
                Ok(frame) => frame,
                Err(error) => {
                    error!("Screenshot capture failed: {:?}", error);
                    return;
                }
            };

            Self::clear_capture_session_shared(
                &Rc::downgrade(&overlays),
                &frame_slot,
                &selection_start,
                &selection_rect,
            );
            *frame_origin.borrow_mut() = DesktopPxPoint::new(virtual_desktop.x, virtual_desktop.y);
            *frame_slot.borrow_mut() = Some(frame);

            let rebuild_overlays = {
                let existing = overlays.borrow();
                existing.len() != displays.len()
                    || existing
                        .iter()
                        .zip(displays.iter())
                        .any(|(surface, display)| {
                            surface.monitor_bounds.x != display.bounds.x
                                || surface.monitor_bounds.y != display.bounds.y
                                || surface.monitor_bounds.width != display.bounds.width
                                || surface.monitor_bounds.height != display.bounds.height
                                || (surface.scale_factor - display.scale_factor.max(1.0)).abs()
                                    > f32::EPSILON
                        })
            };
            if rebuild_overlays {
                let controller_overlays = overlays.clone();
                controller_overlays.borrow_mut().clear();
                for display in displays {
                    let window = Rc::new(OverlayWindow::new().expect("create screenshot overlay"));
                    let monitor_bounds = display.bounds;
                    let scale_factor = display.scale_factor.max(1.0);
                    Self::setup_overlay_callbacks(
                        window.clone(),
                        monitor_bounds,
                        scale_factor,
                        Rc::downgrade(&controller_overlays),
                        frame_slot.clone(),
                        frame_origin.clone(),
                        selection_start.clone(),
                        selection_rect.clone(),
                        capture_service.clone(),
                        self_capture_mode.clone(),
                        longshot_window.clone(),
                        longshot_state.clone(),
                        editor_window.clone(),
                        editor_state.clone(),
                        pin_manager.clone(),
                        ai_window.clone(),
                    );
                    controller_overlays.borrow_mut().push(OverlaySurface {
                        window,
                        monitor_bounds,
                        scale_factor,
                    });
                }
            }

            let frame_guard = frame_slot.borrow();
            let frame = frame_guard.as_ref().expect("captured frame present");
            for surface in overlays.borrow().iter() {
                let monitor_rect = surface.monitor_bounds;
                let Ok(image_rect) = desktop_rect_to_image_rect(
                    monitor_rect,
                    *frame_origin.borrow(),
                    frame.width,
                    frame.height,
                ) else {
                    error!("Monitor lies outside captured virtual desktop frame");
                    continue;
                };
                let Ok(monitor_frame) = frame.crop(image_rect) else {
                    error!("Failed to crop monitor frame for overlay");
                    continue;
                };

                surface.window.window().set_position(PhysicalPosition::new(
                    surface.monitor_bounds.x,
                    surface.monitor_bounds.y,
                ));
                surface.window.window().set_size(PhysicalSize::new(
                    surface.monitor_bounds.width,
                    surface.monitor_bounds.height,
                ));
                surface
                    .window
                    .set_capture_image(frame_to_slint_image(&monitor_frame));
                surface.window.set_has_selection(false);
                surface.window.set_selection_w(0);
                surface.window.set_selection_h(0);
                if let Err(error) = surface.window.show() {
                    error!("Failed to show screenshot overlay: {:?}", error);
                }
            }
            info!("Showing screenshot overlays");
        })
        .unwrap();
    }

    pub fn shutdown(&self) {
        self.pin_manager.close_all();
        self.clear_capture_session();
        *self.longshot_state.borrow_mut() = None;
        *self.editor_state.borrow_mut() = None;
        if let Some(window) = self.longshot_window.borrow().as_ref() {
            let _ = window.hide();
        }
        if let Some(window) = self.editor_window.borrow().as_ref() {
            let _ = window.hide();
        }
        if let Some(window) = self.settings_window.borrow().as_ref() {
            let _ = window.hide();
        }
        if let Some(window) = self.ai_window.borrow().as_ref() {
            let _ = window.hide();
        }
    }

    pub fn trigger_color_picker(&self) {
        self.trigger_screenshot();
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

            let current_key =
                CredentialStorage::read_secret(DEFAULT_TARGET_NAME).unwrap_or_default();
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

        win.set_title_text(if is_translate {
            "截图翻译结果".into()
        } else {
            "OCR 识别结果".into()
        });
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
                match client
                    .translate_image(&png_bytes, &target_lang, cancel)
                    .await
                {
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
