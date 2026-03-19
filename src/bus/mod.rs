mod event;
mod global;

pub use event::definitions::*;
pub use event::{Event, EventDefinition, EventProperties};
pub use global::{GlobalBus, GlobalEvent};

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{RwLock, broadcast};

const BUS_CAPACITY: usize = 256;

type SubscriptionId = u64;
type EventHandler = Box<dyn Fn(&Event) + Send + Sync>;

pub struct Bus {
    directory: String,
    tx: broadcast::Sender<Event>,
    subscriptions: Arc<RwLock<HashMap<String, Vec<(SubscriptionId, EventHandler)>>>>,
    next_subscription_id: Arc<RwLock<u64>>,
}

impl Bus {
    pub fn new(directory: impl Into<String>) -> Self {
        let (tx, _rx) = broadcast::channel(BUS_CAPACITY);
        Self {
            directory: directory.into(),
            tx,
            subscriptions: Arc::new(RwLock::new(HashMap::new())),
            next_subscription_id: Arc::new(RwLock::new(1)),
        }
    }

    pub async fn publish<T: EventProperties>(
        &self,
        definition: &EventDefinition<T>,
        properties: T,
    ) {
        let event = Event::new(definition.event_type, properties);

        tracing::info!("publishing event: {}", definition.event_type);

        let subs = self.subscriptions.read().await;
        if let Some(handlers) = subs.get(definition.event_type) {
            for (_, handler) in handlers {
                handler(&event);
            }
        }

        if let Some(wildcard_handlers) = subs.get("*") {
            for (_, handler) in wildcard_handlers {
                handler(&event);
            }
        }

        let _ = self.tx.send(event.clone());

        GlobalBus::emit(&self.directory, event);
    }

    pub async fn subscribe<T: EventProperties, F>(
        &self,
        definition: &EventDefinition<T>,
        callback: F,
    ) -> impl Fn()
    where
        F: Fn(&Event) + Send + Sync + 'static,
    {
        let event_type = definition.event_type.to_string();
        let id = {
            let mut next_id = self.next_subscription_id.write().await;
            let id = *next_id;
            *next_id += 1;
            id
        };

        tracing::info!("subscribing to event: {}", event_type);

        let handler: EventHandler = Box::new(callback);

        let mut subs = self.subscriptions.write().await;
        subs.entry(event_type.clone())
            .or_insert_with(Vec::new)
            .push((id, handler));

        let subscriptions = Arc::clone(&self.subscriptions);
        move || {
            let event_type = event_type.clone();
            let subscriptions = Arc::clone(&subscriptions);
            tokio::spawn(async move {
                let mut subs = subscriptions.write().await;
                if let Some(handlers) = subs.get_mut(&event_type) {
                    handlers.retain(|(sub_id, _)| *sub_id != id);
                }
            });
        }
    }

    pub async fn subscribe_all<F>(&self, callback: F) -> impl Fn()
    where
        F: Fn(&Event) + Send + Sync + 'static,
    {
        let id = {
            let mut next_id = self.next_subscription_id.write().await;
            let id = *next_id;
            *next_id += 1;
            id
        };

        let handler: EventHandler = Box::new(callback);

        let mut subs = self.subscriptions.write().await;
        subs.entry("*".to_string())
            .or_insert_with(Vec::new)
            .push((id, handler));

        let subscriptions = Arc::clone(&self.subscriptions);
        move || {
            let subscriptions = Arc::clone(&subscriptions);
            tokio::spawn(async move {
                let mut subs = subscriptions.write().await;
                if let Some(handlers) = subs.get_mut("*") {
                    handlers.retain(|(sub_id, _)| *sub_id != id);
                }
            });
        }
    }

    pub fn subscribe_channel(&self) -> broadcast::Receiver<Event> {
        self.tx.subscribe()
    }

    pub fn directory(&self) -> &str {
        &self.directory
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use event::definitions::{SERVER_CONNECTED, ServerConnectedProperties};
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[tokio::test]
    async fn test_publish_subscribe() {
        let bus = Bus::new("/test");
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = Arc::clone(&counter);

        let _unsub = bus
            .subscribe(&SERVER_CONNECTED, move |_| {
                counter_clone.fetch_add(1, Ordering::SeqCst);
            })
            .await;

        bus.publish(&SERVER_CONNECTED, ServerConnectedProperties {})
            .await;
        bus.publish(&SERVER_CONNECTED, ServerConnectedProperties {})
            .await;

        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        assert_eq!(counter.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn test_unsubscribe() {
        let bus = Bus::new("/test");
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = Arc::clone(&counter);

        let unsub = bus
            .subscribe(&SERVER_CONNECTED, move |_| {
                counter_clone.fetch_add(1, Ordering::SeqCst);
            })
            .await;

        bus.publish(&SERVER_CONNECTED, ServerConnectedProperties {})
            .await;
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        assert_eq!(counter.load(Ordering::SeqCst), 1);

        unsub();
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        bus.publish(&SERVER_CONNECTED, ServerConnectedProperties {})
            .await;
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn test_subscribe_all() {
        let bus = Bus::new("/test");
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = Arc::clone(&counter);

        let _unsub = bus
            .subscribe_all(move |_| {
                counter_clone.fetch_add(1, Ordering::SeqCst);
            })
            .await;

        bus.publish(&SERVER_CONNECTED, ServerConnectedProperties {})
            .await;
        bus.publish(
            &PTY_CREATED,
            PtyCreatedProperties {
                info: PtyInfo {
                    id: "test".to_string(),
                    title: "Test".to_string(),
                    command: "bash".to_string(),
                    args: vec![],
                    cwd: "/".to_string(),
                    status: "running".to_string(),
                    pid: 1,
                },
            },
        )
        .await;

        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        assert_eq!(counter.load(Ordering::SeqCst), 2);
    }
}
