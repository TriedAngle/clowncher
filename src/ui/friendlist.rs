use crate::widgets::dragdrop::drag_source;
use eframe::egui;
use kassadin::client::LCU;
use kassadin::types::socket::FriendEvent;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

#[derive(Debug, Clone)]
pub struct Rank {
    pub tier: Option<String>,
    pub division: Option<String>,
}

#[derive(Debug, Clone, PartialEq, PartialOrd)]
#[repr(u8)]
pub enum Status {
    Idle = 0,
    Away = 1,
    Ingame = 2,
    Mobile = 3,
    Offline = 4,
    Other = 5,
}

impl From<Option<String>> for Status {
    fn from(s: Option<String>) -> Self {
        if s.is_none() {
            return Self::Other;
        }
        match s.unwrap().as_str() {
            "away" => Self::Away,
            "mobile" => Self::Mobile,
            "dnd" => Self::Ingame,
            "chat" => Self::Idle,
            "offline" => Self::Offline,
            _ => Self::Other,
        }
    }
}

impl Status {
    pub fn to_color(&self) -> egui::Color32 {
        match *self {
            Status::Other => egui::Color32::BLACK,
            Status::Idle => egui::Color32::LIGHT_GREEN,
            Status::Ingame => egui::Color32::BLUE,
            Status::Away => egui::Color32::LIGHT_RED,
            Status::Mobile => egui::Color32::LIGHT_GRAY,
            Status::Offline => egui::Color32::DARK_GRAY,
        }
    }
}

#[derive(Debug, Clone)]
pub struct FriendListEntry {
    pub name: String,
    pub riot_name: String,
    pub icon: i32,
    pub status: Status,
    pub rank: Rank,
    pub id: String,
    pub summoner_id: i64,
}

impl From<kassadin::types::lcu::chat::Friend> for FriendListEntry {
    fn from(f: kassadin::types::lcu::chat::Friend) -> Self {
        FriendListEntry {
            name: f.game_name,
            riot_name: f.name,
            icon: f.icon,
            status: Status::from(f.availability),
            rank: Rank {
                division: f.lol.ranked_league_division,
                tier: if f.lol.ranked_league_tier.is_some() {
                    Some(f.lol.ranked_league_tier.unwrap())
                } else {
                    None
                },
            },
            id: f.id,
            summoner_id: f.summoner_id,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Sorting {
    Status,
    NameAlphabet,
    NameReverseAlphabet,
    NameSearch(String),
}

impl Default for Sorting {
    fn default() -> Self {
        Self::Status
    }
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct FriendlistConfig {
    pub sorting: Sorting,
}

#[derive(Debug, Default)]
pub struct Friendlist {
    pub config: FriendlistConfig,
    pub friends: Vec<FriendListEntry>,
    pub dragging_friend: Option<FriendListEntry>,
    pub hover_friend: Option<(FriendListEntry, egui::Pos2)>,
}

impl Friendlist {
    pub fn new(lcu: &LCU) -> Friendlist {
        let config = confy::load::<FriendlistConfig>("clowncher/friends").unwrap_or_default();

        if lcu.is_client_running() {
            let mut friendlist = Friendlist {
                friends: Vec::new(),
                config,
                dragging_friend: None,
                hover_friend: None,
            };
            friendlist.reload(lcu);
            friendlist
        } else {
            Default::default()
        }
    }

    pub fn save(&self) {
        confy::store("clowncher/friends", self.config.clone()).unwrap();
    }

    pub fn ui(&mut self, ui: &mut egui::Ui) {
        if let Some(friend) = &self.hover_friend {
            let ctx = ui.ctx();
            egui::Window::new(&friend.0.name)
                .default_pos(friend.1)
                .show(ctx,|ui| {
                ui.label("test");
            });
        }
        self.ui_friends(ui);
    }

    pub fn update(&mut self, update: FriendEvent) {
        let friend = FriendListEntry::from(update);
        if let Some((id, _entry)) = self
            .friends
            .iter()
            .enumerate()
            .find(|(_id, f)| f.id == friend.id)
        {
            self.friends.remove(id);
        }
        self.friends.push(friend);
        self.sort();
    }

    pub fn reload(&mut self, lcu: &LCU) {
        self.friends.clear();
        self.friends = crate::RT
            .block_on(async { lcu.chat().friends().await })
            .unwrap()
            .into_iter()
            .map(|f| FriendListEntry::from(f))
            .collect();
        self.sort();
    }

    pub fn sort(&mut self) {
        match &self.config.sorting {
            Sorting::Status => {
                self.friends
                    .sort_by(|a, b| a.status.partial_cmp(&b.status).unwrap());
            }
            Sorting::NameAlphabet => unimplemented!(),
            Sorting::NameReverseAlphabet => unimplemented!(),
            Sorting::NameSearch(_name) => unimplemented!(),
        }
    }

    pub fn ui_friends(&mut self, ui: &mut egui::Ui) {
        let text_style = egui::TextStyle::Body;
        let row_height = ui.fonts()[text_style].row_height() + 30.0;
        self.dragging_friend = None;
        self.hover_friend = None;
        egui::ScrollArea::vertical()
            .auto_shrink([false, true])
            .show_rows(ui, row_height, self.friends.len(), |ui, row_range| {
                for id in row_range {
                    ui.add_space(10.0);
                    let item_id = egui::Id::new("fl_drag").with(id);
                    drag_source(ui, item_id, |ui| {
                        let friend = self.ui_friend(ui, &self.friends[id]);
                        if let Some(pos) = friend.hover_pos(){
                            self.hover_friend = Some((self.friends[id].clone(), pos));
                        }
                    });
                    ui.add_space(10.0);

                    if ui.memory().is_being_dragged(item_id) {
                        self.dragging_friend = Some(self.friends[id].clone());
                    }
                }
            });
    }

    pub fn ui_friend(&self, ui: &mut egui::Ui, f: &FriendListEntry) -> egui::Response {
        let name =
            egui::Label::new(egui::RichText::new(&f.name).text_style(egui::TextStyle::Button));

        let riot_name =
            egui::Label::new(egui::RichText::new(&f.riot_name).text_style(egui::TextStyle::Small));

        ui.horizontal(|ui| {
            let (response, painter) =
                ui.allocate_painter(egui::Vec2::splat(8.0), egui::Sense::hover());
            let rect = response.rect;
            painter.circle_filled(rect.center(), rect.width() / 2.0, f.status.to_color());
            ui.vertical(|ui| {
                ui.add(name);
                ui.add(riot_name);
            });
        })
        .response
    }
}
