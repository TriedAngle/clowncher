#[macro_use]
extern crate lazy_static;

mod app;
pub mod event;
mod interop;
mod widgets;
mod ui;

use eframe::epi;
use egui::Vec2;
use image::GenericImageView;
use std::collections::HashMap;

pub use app::App;

use tokio::runtime::Runtime;

lazy_static! {
    pub static ref RT: Runtime = Runtime::new().unwrap();
}

#[derive(Debug, Default)]
pub struct TextureManager {
    images: HashMap<String, (egui::Vec2, egui::TextureId)>,
}

impl TextureManager {
    fn add_image(&mut self, name: &str, bytes: &[u8], frame: &epi::Frame) {
        let img = image::load_from_memory(bytes).unwrap();
        let img_buf = img.to_rgba8();
        let size = [img.width() as usize, img.height() as usize];
        let pixels = img_buf.into_vec();
        let image = epi::Image::from_rgba_unmultiplied(size, &pixels);
        let id = frame.alloc_texture(image);
        self.images.insert(
            name.to_string(),
            (egui::Vec2::new(img.width() as f32, img.height() as f32), id),
        );
    }

    fn draw_image(&self, ui: &mut egui::Ui, name: &str, size: Option<egui::Vec2>) {
        let (s, id) = self.images.get(name).unwrap();
        match size {
            None => ui.image(id.clone(), s.clone()),
            Some(size) => ui.image(id.clone(), size),
        };
    }

    fn get_texture_id(&self, name: &str) -> (egui::Vec2, egui::TextureId) {
        self.images.get(name).unwrap().clone()
    }
}
