use crossterm::event::{self, Event as CEvent, KeyCode, KeyEvent};
use std::time::{Duration, Instant};

pub enum Event<I> {
    Input(I),
    Tick,
}

pub struct Events {
    rx: std::sync::mpsc::Receiver<Event<KeyEvent>>,
    _tx: std::sync::mpsc::Sender<Event<KeyEvent>>,
}

impl Events {
    pub fn new(tick_rate: Duration) -> Self {
        let (tx, rx) = std::sync::mpsc::channel();
        let event_tx = tx.clone();
        
        std::thread::spawn(move || {
            let mut last_tick = Instant::now();
            loop {
                let timeout = tick_rate
                    .checked_sub(last_tick.elapsed())
                    .unwrap_or_else(|| Duration::from_secs(0));

                if event::poll(timeout).expect("Failed to poll for events") {
                    if let CEvent::Key(key) = event::read().expect("Failed to read event") {
                        event_tx.send(Event::Input(key)).expect("Failed to send key event");
                    }
                }

                if last_tick.elapsed() >= tick_rate {
                    event_tx.send(Event::Tick).expect("Failed to send tick event");
                    last_tick = Instant::now();
                }
            }
        });

        Events { rx, _tx: tx }
    }

    pub fn next(&self) -> Result<Event<KeyEvent>, std::sync::mpsc::RecvError> {
        self.rx.recv()
    }
}

pub fn handle_key_event(key: KeyCode) -> Option<Action> {
    match key {
        KeyCode::Char('q') => Some(Action::Quit),
        KeyCode::Char('s') => Some(Action::ToggleSort),
        KeyCode::Down | KeyCode::Char('j') => Some(Action::MoveSelection(1)),
        KeyCode::Up | KeyCode::Char('k') => Some(Action::MoveSelection(-1)),
        KeyCode::Right | KeyCode::Enter => Some(Action::NavigateIn),
        KeyCode::Left | KeyCode::Backspace => Some(Action::NavigateOut),
        _ => None,
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Action {
    Quit,
    ToggleSort,
    MoveSelection(isize),
    NavigateIn,
    NavigateOut,
}
