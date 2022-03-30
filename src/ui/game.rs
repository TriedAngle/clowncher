use eframe::egui;

use kassadin::client::LCU;
use kassadin::types::lcu::consts::{Position, QueueId, Tier};
use kassadin::types::lcu::lobby::{Member, PositionPreference};
use serde::{Deserialize, Serialize};

use kassadin::types::consts::Division;
use kassadin::types::lcu::ranked::RankedStatus;
use std::string::ToString;

use crate::widgets::dragdrop::drop_target;
use crate::ui::friendlist::Friendlist;
use crate::TextureManager;

#[derive(Debug, PartialEq)]
pub enum SearchState {
    None,
    Lobby,
    Searching,
    Found,
    ChampSelect,
    InGame,
    AfterGameLobby,
    Error,
}

impl Default for SearchState {
    fn default() -> Self {
        Self::None
    }
}

#[derive(Debug, Clone)]
pub struct LobbyMemberRanked {
    pub division: Division,
    pub tier: Tier,
    pub wins: i64,
    pub losses: i64,
    pub lp: i64,
}

#[derive(Debug, Clone)]
pub struct LobbyMember {
    pub name: String,
    pub ranked: LobbyMemberRanked,
    pub autofillable: bool,
    pub leader: bool,
    pub positions: PositionPreference,
    pub summoner_id: i64,
}

impl LobbyMember {
    pub fn from(m: Member, ranked: RankedStatus) -> Self {
        let ranked = LobbyMemberRanked {
            division: ranked.queue_map.ranked_solo.division,
            tier: ranked.queue_map.ranked_solo.tier,
            wins: ranked.queue_map.ranked_solo.wins,
            losses: ranked.queue_map.ranked_solo.losses,
            lp: ranked.queue_map.ranked_solo.league_points,
        };
        let positions = PositionPreference {
            first_preference: m.first_position_preference,
            second_preference: m.second_position_preference,
        };
        Self {
            name: m.summoner_name.unwrap_or_default(),
            ranked,
            autofillable: m.auto_fill_eligible.unwrap_or_default(),
            leader: m.is_leader.unwrap_or_default(),
            positions,
            summoner_id: m.summoner_id,
        }
    }
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct GameConfig {
    pub auto_accept: bool,
}

#[derive(Debug)]
pub struct Game {
    pub config: GameConfig,
    pub queue_id: Option<QueueId>,
    pub search_sate: SearchState,
    pub queue_timer: Option<f64>,
    pub estimated_queue_time: Option<f64>,
    pub positions: PositionPreference,
    pub members: Vec<LobbyMember>,
    pub select_second: bool,
}

impl Default for Game {
    fn default() -> Self {
        let config = confy::load::<GameConfig>("clowncher/game").unwrap_or_default();

        Self {
            queue_id: None,
            search_sate: SearchState::None,
            queue_timer: None,
            estimated_queue_time: None,
            positions: Default::default(),
            members: vec![],
            select_second: false,
            config,
        }
    }
}

impl Game {
    pub fn save(&self) {
        confy::store("clowncher/game", self.config.clone()).unwrap();
    }

    pub fn ui(
        &mut self,
        ui: &mut egui::Ui,
        lcu: &LCU,
        textures: &TextureManager,
        friendlist: &Friendlist,
    ) {
        ui.vertical(|ui| {
            self.ui_lobby(ui, lcu, textures, friendlist);
            ui.with_layout(egui::Layout::left_to_right(), |ui| {
                self.ui_selection(ui, lcu, textures);
            });
        });
    }

    pub fn update_members(&mut self, members: Vec<LobbyMember>) {
        self.members = members;
    }

    fn ui_lobby(
        &mut self,
        ui: &mut egui::Ui,
        lcu: &LCU,
        textures: &TextureManager,
        friendlist: &Friendlist,
    ) {
        match self.search_sate {
            SearchState::None => {
                ui.label(
                    egui::RichText::new("Not in lobby")
                        .text_style(egui::TextStyle::Heading)
                        .color(crate::ui::colors::INDIGO_A700),
                );
            }
            SearchState::Lobby => {
                let response = drop_target(ui, friendlist.dragging_friend.is_some(), |ui| {
                    self.ui_members(ui, lcu, textures);
                })
                .response;

                if ui.memory().is_anything_being_dragged()
                    && friendlist.dragging_friend.is_some()
                    && response.hovered()
                    && ui.input().pointer.any_released()
                {
                    let friend = friendlist.dragging_friend.as_ref().unwrap();
                    println!("friend: {:?}", friend);
                    match crate::RT
                        .block_on(async { lcu.lobby().invite_member(friend.summoner_id).await })
                    {
                        Ok(_response) => {}
                        Err(_e) => {}
                    }
                }
            }
            SearchState::Searching => {
                self.ui_members(ui, lcu, textures);
            }
            SearchState::Found => {}
            SearchState::ChampSelect => {}
            SearchState::InGame => {}
            SearchState::AfterGameLobby => {
                self.ui_after_game_lobby(ui, lcu);
            }
            SearchState::Error => {}
        }
    }

    fn ui_members(&mut self, ui: &mut egui::Ui, lcu: &LCU, textures: &TextureManager) {
        for member in &self.members {
            ui.add_space(5.0);
            ui.horizontal(|ui| {
                if ui
                    .add(egui::Button::new(
                        egui::RichText::new("X").color(crate::ui::colors::RED_A500),
                    ))
                    .clicked()
                {
                    crate::RT
                        .block_on(async { lcu.lobby().kick_member(member.summoner_id).await })
                        .unwrap();
                }
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new(&member.name).text_style(egui::TextStyle::Button),
                        );
                        if member.leader {
                            textures.draw_image(ui, "leader", Some(egui::Vec2::new(25.0, 25.0)));
                        }
                        if member.autofillable {
                            textures.draw_image(
                                ui,
                                "autofillable",
                                Some(egui::Vec2::new(25.0, 25.0)),
                            );
                        }
                        if let Some(pos) = member.positions.first_preference {
                            if pos != Position::UNSELECTED {
                                textures.draw_image(
                                    ui,
                                    &pos.to_string().to_lowercase(),
                                    Some(egui::Vec2::new(25.0, 25.0)),
                                );
                            }
                        }
                        if let Some(pos) = member.positions.second_preference {
                            if pos != Position::UNSELECTED {
                                textures.draw_image(
                                    ui,
                                    &pos.to_string().to_lowercase(),
                                    Some(egui::Vec2::new(25.0, 25.0)),
                                );
                            }
                        }
                    });
                    ui.add_space(2.0);
                    // TODO: add riot api to display losses because lcu can't do that ?XD
                    let ranked_label = format!(
                        "{} {} | {} LP | {} wins",
                        member.ranked.tier.to_string(),
                        member.ranked.division.to_string(),
                        member.ranked.lp.to_string(),
                        member.ranked.wins.to_string(),
                    );
                    ui.label(egui::RichText::new(&ranked_label));
                    ui.add(egui::Separator::default());
                });
            });
        }
    }

    fn ui_selection(&mut self, ui: &mut egui::Ui, lcu: &LCU, textures: &TextureManager) {
        match self.search_sate {
            SearchState::None => {
                self.ui_selection_none(ui, lcu, textures);
            }
            SearchState::Lobby => {
                self.ui_selection_lobby(ui, lcu, textures);
            }
            SearchState::Searching => {
                self.ui_selection_searching(ui, lcu);
            }
            SearchState::Found => {
                self.ui_selection_found(ui, lcu);
            }
            SearchState::ChampSelect => {
                self.ui_selection_champ_select(ui, lcu);
            }
            SearchState::InGame => {
                self.ui_selection_in_game();
            }
            SearchState::AfterGameLobby => {
                self.ui_selection_lobby(ui, lcu, textures);
            }
            SearchState::Error => {
                self.ui_selection_error(ui, lcu);
            }
        }
    }

    pub fn ui_game_button(
        &mut self,
        ui: &mut egui::Ui,
        lcu: &LCU,
        t: QueueId,
        text: impl Into<String>,
    ) {
        if self.queue_id.is_some() && self.queue_id.unwrap() == t {
            let button = ui.add(
                egui::Button::new(
                    egui::RichText::new(text)
                        .text_style(egui::TextStyle::Button)
                        .color(egui::Color32::WHITE),
                )
                .fill(crate::ui::colors::DEEP_PURPLE_A400),
            );
            if button.clicked() {
                self.queue_id = None;
                crate::RT
                    .block_on(async { lcu.lobby().leave_lobby().await })
                    .unwrap();
            }
        } else {
            let button = ui.add(
                egui::Button::new(
                    egui::RichText::new(text)
                        .text_style(egui::TextStyle::Button)
                        .color(egui::Color32::WHITE),
                )
                .fill(crate::ui::colors::BLUE_A400),
            );
            if button.clicked() {
                self.queue_id = Some(t);
                self.join_lobby(lcu);
            }
        }
    }

    pub fn join_lobby(&mut self, lcu: &LCU) {
        match crate::RT.block_on(async {
            lcu.lobby()
                .join_lobby(self.queue_id.unwrap())
                .await
                .and(lcu.lobby().set_roles(&self.positions).await)
        }) {
            Ok(_response) => {}
            Err(_e) => {}
        }
    }

    pub fn update_position(&mut self, lcu: &LCU, position: Position) {
        if self.positions.first_preference.is_some()
            && self.positions.first_preference.unwrap() == position
        {
            self.positions.first_preference = None;
            self.select_second = false;
        } else if self.positions.second_preference.is_some()
            && self.positions.second_preference.unwrap() == position
        {
            self.positions.second_preference = None;
            self.select_second = self.positions.first_preference.is_some();
        } else if !self.select_second {
            self.positions.first_preference = Some(position);
            self.select_second = true;
        } else if self.select_second {
            self.positions.second_preference = Some(position);
            self.select_second = false;
        }

        if self.search_sate == SearchState::Lobby {
            match crate::RT.block_on(async { lcu.lobby().set_roles(&self.positions).await }) {
                Ok(_response) => {}
                Err(_e) => {}
            }
        }
    }

    fn ui_selection_none(&mut self, ui: &mut egui::Ui, lcu: &LCU, textures: &TextureManager) {
        self.join_lobby_buttons(ui, lcu);

        ui.add(
            egui::Button::new(
                egui::RichText::new("Search")
                    .text_style(egui::TextStyle::Heading)
                    .color(egui::Color32::WHITE),
            )
            .fill(crate::ui::colors::GRAY_A500)
            .sense(egui::Sense {
                click: false,
                drag: false,
                focusable: false,
            }),
        );

        self.ui_role_buttons(ui, lcu, textures);
    }

    fn ui_selection_lobby(&mut self, ui: &mut egui::Ui, lcu: &LCU, textures: &TextureManager) {
        self.join_lobby_buttons(ui, lcu);

        let button = ui.add(
            egui::Button::new(
                egui::RichText::new("Search")
                    .text_style(egui::TextStyle::Heading)
                    .color(crate::ui::colors::YELLOW_A800),
            )
            .fill(crate::ui::colors::GREEN_A400),
        );

        self.ui_role_buttons(ui, lcu, textures);

        if button.clicked() {
            crate::RT
                .block_on(async { lcu.lobby().start_queue().await })
                .unwrap();
        }
    }

    fn ui_selection_searching(&mut self, ui: &mut egui::Ui, lcu: &LCU) {
        let button = ui.add(
            egui::Button::new(
                egui::RichText::new("Cancel")
                    .text_style(egui::TextStyle::Heading)
                    .color(crate::ui::colors::YELLOW_A800),
            )
            .fill(crate::ui::colors::INDIGO_A700),
        );

        if button.clicked() {
            crate::RT
                .block_on(async { lcu.lobby().stop_queue().await })
                .unwrap();
        }
    }

    fn ui_selection_found(&mut self, ui: &mut egui::Ui, lcu: &LCU) {
        let accept_button = ui.add(
            egui::Button::new(
                egui::RichText::new("Accept")
                    .text_style(egui::TextStyle::Heading)
                    .color(crate::ui::colors::YELLOW_A800),
            )
            .fill(crate::ui::colors::INDIGO_A700),
        );

        let decline_button = ui.add(egui::Button::new(
            egui::RichText::new("Decline")
                .text_style(egui::TextStyle::Heading)
                .color(crate::ui::colors::RED_A500),
        ));

        if accept_button.clicked() {
            crate::RT
                .block_on(async { lcu.matchmaking().accept().await })
                .unwrap();
        }

        if decline_button.clicked() {
            crate::RT
                .block_on(async { lcu.matchmaking().decline().await })
                .unwrap();
        }
    }

    fn ui_selection_champ_select(&mut self, ui: &mut egui::Ui, lcu: &LCU) {
        let button = ui.add(
            egui::Button::new(
                egui::RichText::new("Dodge")
                    .text_style(egui::TextStyle::Heading)
                    .color(egui::Color32::WHITE),
            )
            .fill(crate::ui::colors::RED_A500),
        );

        let opgg_button = ui.add(
            egui::Button::new(
                egui::RichText::new("op.gg")
                    .text_style(egui::TextStyle::Heading)
                    .color(egui::Color32::WHITE),
            )
                .fill(crate::ui::colors::BLUE_A400)
        );




        if button.clicked() {
            crate::RT
                .block_on(async { lcu.login().dodge_lobby().await })
                .unwrap();
        }
    }

    fn ui_selection_in_game(&self) {}

    fn ui_after_game_lobby(&mut self, ui: &mut egui::Ui, _lcu: &LCU) {
        let button = ui.add(
            egui::Button::new(
                egui::RichText::new("Play Again")
                    .text_style(egui::TextStyle::Heading)
                    .color(crate::ui::colors::YELLOW_A800),
            )
            .fill(crate::ui::colors::INDIGO_A700),
        );

        if button.clicked() {
            self.search_sate = SearchState::None;
        }
    }

    fn ui_selection_error(&mut self, ui: &mut egui::Ui, _lcu: &LCU) {
        let button = ui.add(
            egui::Button::new(
                egui::RichText::new("Home")
                    .text_style(egui::TextStyle::Heading)
                    .color(egui::Color32::GRAY),
            )
            .fill(egui::Color32::DARK_GRAY),
        );

        if button.clicked() {
            self.search_sate = SearchState::None;
        }
    }

    fn join_lobby_buttons(&mut self, ui: &mut egui::Ui, lcu: &LCU) {
        egui::Grid::new("lobby_buttons")
            .spacing(egui::Vec2::new(5.0, 5.0))
            .show(ui, |ui| {
                self.ui_game_button(ui, lcu, QueueId::Solo, "Solo");
                self.ui_game_button(ui, lcu, QueueId::Draft, "Draft");
                ui.end_row();
                self.ui_game_button(ui, lcu, QueueId::Flex, "Flex");
                self.ui_game_button(ui, lcu, QueueId::Blind, "Blind");
                ui.end_row();
                self.ui_game_button(ui, lcu, QueueId::Clash, "Clash");
                self.ui_game_button(ui, lcu, QueueId::Aram, "Aram");
                ui.end_row();
            });
    }

    fn ui_role_buttons(&mut self, ui: &mut egui::Ui, lcu: &LCU, textures: &TextureManager) {
        egui::Grid::new("roles")
            .spacing(egui::Vec2::splat(5.0))
            .show(ui, |ui| {
                self.ui_role_button(ui, lcu, Position::TOP, "top", textures);
                self.ui_role_button(ui, lcu, Position::JUNGLE, "jungle", textures);
                self.ui_role_button(ui, lcu, Position::MIDDLE, "middle", textures);
                ui.end_row();
                self.ui_role_button(ui, lcu, Position::BOTTOM, "bottom", textures);
                self.ui_role_button(ui, lcu, Position::UTILITY, "utility", textures);
                self.ui_role_button(ui, lcu, Position::FILL, "fill", textures);
            });
    }

    fn ui_role_button(
        &mut self,
        ui: &mut egui::Ui,
        lcu: &LCU,
        position: Position,
        img: &str,
        textures: &TextureManager,
    ) {
        let image = textures.get_texture_id(img);

        let mut button = egui::ImageButton::new(image.1, egui::Vec2::new(40.0, 40.0));

        if self.positions.first_preference.is_some()
            && self.positions.first_preference.unwrap() == position
        {
            button = button.tint(crate::ui::colors::LIGHT_BLUE_A400)
        } else if self.positions.second_preference.is_some()
            && self.positions.second_preference.unwrap() == position
        {
            button = button.tint(crate::ui::colors::CYAN_A200)
        }

        let response = ui.add(button);

        if response.clicked() {
            self.update_position(lcu, position);
        }
    }
}
