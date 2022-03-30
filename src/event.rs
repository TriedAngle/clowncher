use kassadin::types::socket::LeagueEvent;

pub enum Event {
    CreateSocket(i32),
    SocketCreated(i32),
    DeleteSocket(i32),
    ReadSocket(i32),
    LeagueEvent(i32, LeagueEvent),
    LeagueEventEmpty(i32),
    Ready,
}
