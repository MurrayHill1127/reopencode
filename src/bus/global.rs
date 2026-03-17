use super::event::Event;
use std::sync::Arc;
use tokio::sync::broadcast;

const GLOBAL_BUS_CAPACITY: usize = 1024;

static GLOBAL_BUS: once_cell::sync::Lazy<Arc<broadcast::Sender<GlobalEvent>>> = 
    once_cell::sync::Lazy::new(|| {
        let (tx, _rx) = broadcast::channel(GLOBAL_BUS_CAPACITY);
        Arc::new(tx)
    });

#[derive(Debug, Clone)]
pub struct GlobalEvent {
    pub directory: String,
    pub payload: Event,
}

pub struct GlobalBus;

impl GlobalBus {
    pub fn emit(directory: &str, event: Event) {
        let global_event = GlobalEvent {
            directory: directory.to_string(),
            payload: event,
        };
        let _ = GLOBAL_BUS.send(global_event);
    }

    pub fn subscribe() -> broadcast::Receiver<GlobalEvent> {
        GLOBAL_BUS.subscribe()
    }
}