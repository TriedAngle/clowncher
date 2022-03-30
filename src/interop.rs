use crate::event::Event;
use crate::ui::account::Account;
use crate::ui::friendlist::Friendlist;
use crate::ui::game::{Game, LobbyMember, SearchState};
use crossbeam::channel::{Receiver, Sender};
use eframe::epi;
use kassadin::client::{WebSocket, LCU};
use kassadin::routes;

use kassadin::types::socket::{EventType, GameFlowPhase, LeagueEvent, LeagueEventKind};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::RwLock;
use websocket::OwnedMessage;

pub fn spawn_interop_thread(sender: Sender<Event>, receiver: Receiver<Event>, lcu: &LCU) {
    let sockets = RwLock::new(HashMap::<i32, Receiver<LeagueEvent>>::new());
    let lcu = lcu.clone();
    std::thread::spawn(move || {
        while let Ok(event) = receiver.recv() {
            match event {
                Event::CreateSocket(idx) => {
                    let websocket = { lcu.socket().build() };

                    let (send, recv) = crossbeam::channel::unbounded();
                    {
                        let mut s = sockets.write().unwrap();
                        s.insert(idx, recv);
                    }
                    std::thread::spawn(move || {
                        handle_socket_thread(websocket, send);
                    });

                    sender.send(Event::SocketCreated(idx)).unwrap();
                }
                Event::ReadSocket(idx) => {
                    let sockets = sockets.read().unwrap();
                    let events = sockets.get(&idx).unwrap();
                    if let Ok(event) = events.recv() {
                        sender.send(Event::LeagueEvent(idx, event)).unwrap();
                    }
                }
                _ => {}
            }
            sender.send(Event::Ready).unwrap();
        }
    });
}

fn handle_socket_thread(mut socket: WebSocket, send: Sender<LeagueEvent>) {
    socket.start().unwrap();
    for message in socket.client.incoming_messages() {
        let message = message.unwrap();
        match message {
            OwnedMessage::Text(message) => {
                if let Ok(val) = serde_json::from_str::<Value>(&message) {
                    let mut event = match serde_json::from_value::<
                        kassadin::types::socket::LeagueEvent,
                    >(val[2].clone())
                    {
                        Ok(event) => event,
                        Err(_) => LeagueEvent::default(),
                    };

                    if val[2].get("uri").is_some() {
                        let uri = val[2]["uri"].as_str().unwrap();
                        match uri {
                            routes::matchmaking::SEARCH
                            | routes::lobby::team_builder::MATCHMAKING => {
                                event.kind = Some(LeagueEventKind::Queue(None));
                                match serde_json::from_value::<kassadin::types::socket::QueueEvent>(
                                    val[2]["data"].clone(),
                                ) {
                                    Ok(kind) => {
                                        event.kind = Some(LeagueEventKind::Queue(Some(kind)))
                                    }
                                    Err(_) => {}
                                }
                                if uri == routes::lobby::team_builder::MATCHMAKING {}
                                send.send(event).unwrap();
                            }
                            routes::game_flow::SESSION => {
                                event.kind = Some(LeagueEventKind::GameFlow(None));
                                match serde_json::from_value::<kassadin::types::socket::GameFlowEvent>(
                                    val[2]["data"].clone(),
                                ) {
                                    Ok(kind) => {
                                        event.kind = Some(LeagueEventKind::GameFlow(Some(kind)))
                                    }
                                    Err(e) => {
                                        println!("error: {:?}", e);
                                    }
                                }
                                send.send(event).unwrap();
                            }
                            routes::lobby::LOBBY => {
                                event.kind = Some(LeagueEventKind::Lobby(None));
                                match serde_json::from_value::<kassadin::types::socket::LobbyEvent>(
                                    val[2]["data"].clone(),
                                ) {
                                    Ok(kind) => {
                                        event.kind = Some(LeagueEventKind::Lobby(Some(kind)))
                                    }
                                    Err(e) => {
                                        println!("error: {:?}", e);
                                    }
                                }
                                send.send(event).unwrap();
                            }
                            // match things that require if 😔
                            _ => {
                                if uri.contains(routes::chat::FRIENDS) {
                                    event.kind = Some(LeagueEventKind::Friend(None));
                                    match serde_json::from_value::<
                                        kassadin::types::socket::FriendEvent,
                                    >(
                                        val[2]["data"].clone()
                                    ) {
                                        Ok(kind) => {
                                            event.kind = Some(LeagueEventKind::Friend(Some(kind)))
                                        }
                                        Err(e) => {
                                            println!("error: {:?}", e);
                                        }
                                    }
                                    send.send(event).unwrap();
                                }
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }
}

pub fn match_events(
    receiver: &mut Receiver<Event>,
    sender: &mut Sender<Event>,
    ctx: &egui::CtxRef,
    _frame: &epi::Frame,
    lcu: &LCU,
    game: &mut Game,
    _account: &mut Account,
    friendlist: &mut Friendlist,
) {
    while let Ok(event) = receiver.recv_deadline(std::time::Instant::now()) {
        match event {
            Event::Ready => {
                sender.send(Event::ReadSocket(0)).unwrap();
            }
            Event::SocketCreated(idx) => {
                println!("Created Socket: {}", idx);
                sender.send(Event::ReadSocket(idx)).unwrap();
            }
            Event::LeagueEvent(idx, event) => {
                match event.kind {
                    None => panic!("this shouldn't be the case lol"),
                    Some(kind) => match kind {
                        LeagueEventKind::Queue(queue_event) => {
                            if event.event_type.is_some() {
                                match event.event_type.unwrap() {
                                    EventType::Update => {
                                        if let Some(queue_event) = queue_event {
                                            game.queue_timer = queue_event.time_in_queue;
                                            game.estimated_queue_time =
                                                queue_event.estimated_queue_time;
                                        }
                                    }
                                    EventType::Delete => {
                                        game.estimated_queue_time = None;
                                        game.queue_timer = None;
                                    }
                                    _ => {}
                                }
                            }
                        }
                        LeagueEventKind::Lobby(lobby) => {
                            if let Some(lobby) = lobby {
                                if let Some(members) = &lobby.members {
                                    let mut m = vec![];
                                    for member in members {
                                        if let Some(puuid) = &member.puuid {
                                            match crate::RT
                                                .block_on(async { lcu.ranked().stats(puuid).await })
                                            {
                                                Ok(status) => {
                                                    m.push(LobbyMember::from(
                                                        member.clone(),
                                                        status,
                                                    ));
                                                }
                                                Err(e) => {
                                                    println!("lobby {:?}", e)
                                                }
                                            }
                                        }
                                    }
                                    game.update_members(m);
                                }
                            }
                        }
                        LeagueEventKind::ChampSelect => {}
                        LeagueEventKind::GameFlow(flow_event) => {
                            if let Some(flow_event) = flow_event {
                                if let Some(phase) = flow_event.phase {
                                    match phase {
                                        GameFlowPhase::Lobby => {
                                            game.search_sate = SearchState::Lobby;
                                        }
                                        GameFlowPhase::Matchmaking => {
                                            game.search_sate = SearchState::Searching;
                                        }
                                        GameFlowPhase::ReadyCheck => {
                                            game.search_sate = SearchState::Found;
                                        }
                                        GameFlowPhase::ChampSelect => {
                                            game.search_sate = SearchState::ChampSelect;
                                        }
                                        GameFlowPhase::GameStart => {
                                            game.search_sate = SearchState::InGame;
                                        }
                                        GameFlowPhase::InProgress => {
                                            game.search_sate = SearchState::InGame;
                                        }
                                        GameFlowPhase::PreEndOfGame => {}
                                        GameFlowPhase::EndOfGame => {}
                                        GameFlowPhase::WaitingForStats => {
                                            game.search_sate = SearchState::AfterGameLobby;
                                        }
                                        GameFlowPhase::TerminatedInError => {
                                            game.search_sate = SearchState::Error;
                                        }
                                        GameFlowPhase::None => {
                                            game.search_sate = SearchState::None;
                                        }
                                    }
                                }
                            }
                        }
                        LeagueEventKind::Friend(friend) => {
                            if let Some(friend) = friend {
                                friendlist.update(friend);
                            }
                        }
                    },
                }
                ctx.request_repaint();
                sender.send(Event::ReadSocket(idx)).unwrap();
            }
            Event::LeagueEventEmpty(idx) => {
                sender.send(Event::ReadSocket(idx)).unwrap();
            }
            _ => {}
        }
    }
}
