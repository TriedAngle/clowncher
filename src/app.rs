use crate::event::Event;
use crate::interop::spawn_interop_thread;
use crate::ui::account::Account;
use crate::ui::friendlist::Friendlist;
use crate::ui::game::{Game, SearchState};
use eframe::{egui, epi};
use kassadin::client::LCU;

use crate::TextureManager;
use std::time::Duration;

pub struct App {
    game: Game,
    account: Account,
    friendlist: Friendlist,
    instances: Vec<Instance>,
    lcu: kassadin::client::LCU,
    sender: crossbeam::channel::Sender<Event>,
    receiver: crossbeam::channel::Receiver<Event>,
    textures: TextureManager,
    show_window: bool,
}

impl Default for App {
    fn default() -> Self {
        let (app_lol_send, app_lol_recv) = crossbeam::channel::unbounded();
        let (lol_app_send, lol_app_recv) = crossbeam::channel::unbounded();

        let lcu = LCU::new();

        spawn_interop_thread(lol_app_send, app_lol_recv, &lcu);

        Self {
            game: Default::default(),
            sender: app_lol_send,
            receiver: lol_app_recv,
            friendlist: Friendlist::new(&lcu),
            account: Default::default(),
            lcu,
            instances: vec![],
            textures: Default::default(),
            show_window: true,
        }
    }
}

impl App {
    fn load_images(&mut self, frame: &epi::Frame) {
        self.textures.add_image(
            "top",
            include_bytes!("../assets/roles/icon-position-top.png"),
            frame,
        );
        self.textures.add_image(
            "jungle",
            include_bytes!("../assets/roles/icon-position-jungle.png"),
            frame,
        );
        self.textures.add_image(
            "middle",
            include_bytes!("../assets/roles/icon-position-middle.png"),
            frame,
        );
        self.textures.add_image(
            "bottom",
            include_bytes!("../assets/roles/icon-position-bottom.png"),
            frame,
        );
        self.textures.add_image(
            "utility",
            include_bytes!("../assets/roles/icon-position-utility.png"),
            frame,
        );
        self.textures.add_image(
            "fill",
            include_bytes!("../assets/roles/icon-position-fill.png"),
            frame,
        );
        self.textures.add_image(
            "autofillable",
            include_bytes!("../assets/autofillable.png"),
            frame,
        );
        self.textures.add_image(
            "leader",
            include_bytes!("../assets/captain-icon-crown.png"),
            frame,
        );
    }

    fn configure_fonts(&self, ctx: &egui::CtxRef) {
        let mut fonts = egui::FontDefinitions::default();

        fonts.font_data.insert(
            "Custom".to_string(),
            egui::FontData::from_static(include_bytes!("../assets/BeVietnamPro-Regular.ttf")),
        );

        fonts.family_and_size.insert(
            egui::TextStyle::Heading,
            (egui::FontFamily::Proportional, 35.0),
        );
        fonts.family_and_size.insert(
            egui::TextStyle::Button,
            (egui::FontFamily::Proportional, 25.0),
        );

        fonts.family_and_size.insert(
            egui::TextStyle::Body,
            (egui::FontFamily::Proportional, 20.0),
        );

        fonts.family_and_size.insert(
            egui::TextStyle::Small,
            (egui::FontFamily::Proportional, 15.0),
        );

        fonts
            .fonts_for_family
            .get_mut(&egui::FontFamily::Proportional)
            .unwrap()
            .insert(0, "Custom".to_string());

        fonts
            .fonts_for_family
            .get_mut(&egui::FontFamily::Monospace)
            .unwrap()
            .push("Custom".to_string());

        ctx.set_fonts(fonts)
    }
}

impl epi::App for App {
    fn update(&mut self, ctx: &egui::CtxRef, frame: &epi::Frame) {
        let Self {
            game,
            friendlist,
            instances,
            lcu,
            sender,
            receiver,
            account,
            textures,
            show_window,
        } = self;

        crate::interop::match_events(receiver, sender, ctx, frame, lcu, game, account, friendlist);

        egui::SidePanel::left("left_panel")
            .width_range(260.0..=260.0)
            .show(ctx, |ui| {
                // TODO: Each instance should have a place here
            });

        egui::CentralPanel::default()
            .show(ctx, |ui| {
            game.ui(ui, lcu, textures, friendlist);

            if game.search_sate == SearchState::Searching {
                ui.horizontal(|ui| {
                    if let Some(timer) = game.queue_timer {
                        ui.label(format!("{}", timer));
                    }
                    if let Some(estimated) = game.estimated_queue_time {
                        ui.label(format!("{}", estimated));
                    }
                });
            }
        });

        egui::SidePanel::right("right_panel")
            .width_range(260.0..=260.0)
            .show(ctx, |ui| {
                friendlist.ui(ui);
            });

        egui::Window::new("Window")
            .drag_bounds(ctx.used_rect())
            .open(show_window)
            .show(ctx, |ui| {
                ui.label("Windows can be moved by dragging them.");
                ui.label("They are automatically sized based on contents.");
                ui.label("You can turn on resizing and scrolling if you like.");
                ui.label("You would normally chose either panels OR windows.");
            });
    }

    fn setup(
        &mut self,
        ctx: &egui::CtxRef,
        frame: &epi::Frame,
        _storage: Option<&dyn epi::Storage>,
    ) {
        frame.set_window_size(egui::Vec2::new(1280.0, 720.0));
        self.configure_fonts(ctx);
        self.load_images(frame);
        let lock = frame.0.lock().unwrap();
        let repaint_signal = lock.repaint_signal.clone();
        std::thread::spawn(move || loop {
            std::thread::sleep(Duration::from_millis(100));
            repaint_signal.request_repaint();
        });
        self.sender.send(Event::CreateSocket(0)).unwrap();
    }

    fn save(&mut self, _storage: &mut dyn epi::Storage) {
        self.game.save();
        self.friendlist.save();
    }

    fn name(&self) -> &str {
        "Clowncher"
    }
}

pub struct Instance {
    pub id: i32,
    pub game: Game,
    pub account: Account,
    pub lcu: LCU,
}
