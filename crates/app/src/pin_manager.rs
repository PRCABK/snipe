use domain::Frame;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tracing::info;
use ui_slint::{frame_to_slint_image, PinWindow};

pub struct PinnedItem {
    pub id: String,
    pub frame: Arc<Frame>,
    pub window: PinWindow,
}

#[derive(Clone, Default)]
pub struct PinManager {
    pins: Arc<Mutex<HashMap<String, PinnedItem>>>,
}

impl PinManager {
    pub fn new() -> Self {
        Self {
            pins: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn spawn_pin(&self, frame: Frame) -> String {
        let id = uuid::Uuid::new_v4().to_string();
        let frame_arc = Arc::new(frame);
        let slint_img = frame_to_slint_image(&frame_arc);

        let window = PinWindow::new().unwrap();
        window.set_pin_image(slint_img);
        window.set_opacity_level(1.0);
        window.set_is_always_on_top(true);

        let id_clone = id.clone();
        let pins_clone = self.pins.clone();
        window.on_close_clicked(move || {
            let mut guard = pins_clone.lock().unwrap();
            if let Some(item) = guard.remove(&id_clone) {
                let _ = item.window.hide();
                info!("Closed pin window {}", id_clone);
            }
        });

        let frame_clone = frame_arc.clone();
        window.on_copy_clicked(move || {
            let _ = imaging::copy_frame_to_clipboard(&frame_clone);
        });

        let frame_clone_save = frame_arc.clone();
        window.on_save_clicked(move || {
            let cfg = config::AppConfig::load();
            let save_dir = std::path::PathBuf::from(&cfg.output.default_save_dir);
            let _ = imaging::save_frame_atomic(
                &frame_clone_save,
                &save_dir,
                &cfg.output.filename_template,
                &cfg.output.default_format,
                cfg.output.jpeg_quality,
            );
        });

        window.show().unwrap();

        let item = PinnedItem {
            id: id.clone(),
            frame: frame_arc,
            window,
        };

        self.pins.lock().unwrap().insert(id.clone(), item);
        info!("Spawned pin window with id {}", id);
        id
    }

    pub fn close_all(&self) {
        let mut guard = self.pins.lock().unwrap();
        for (_, item) in guard.drain() {
            let _ = item.window.hide();
        }
    }
}
