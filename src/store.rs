use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use tokio::sync::broadcast;
use uuid::Uuid;

use crate::message::Message;

/// Events broadcast when the store changes. Used by the SSE stream and the CLI
/// printer task.
#[derive(Debug, Clone)]
pub enum StoreEvent {
    Message(Box<Message>),
    Cleared,
    Deleted(Uuid),
}

/// Bounded in-memory ring buffer of captured messages with a broadcast channel
/// that fans out new arrivals to every subscriber (CLI + SSE clients).
pub struct MessageStore {
    inner: Mutex<VecDeque<Message>>,
    capacity: usize,
    tx: broadcast::Sender<StoreEvent>,
}

impl MessageStore {
    pub fn new(capacity: usize) -> Arc<Self> {
        let cap = capacity.max(1);
        let (tx, _) = broadcast::channel(256);
        Arc::new(Self {
            inner: Mutex::new(VecDeque::with_capacity(cap)),
            capacity: cap,
            tx,
        })
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }

    pub fn push(&self, msg: Message) {
        {
            let mut q = self.inner.lock().expect("store poisoned");
            if q.len() >= self.capacity {
                q.pop_front();
            }
            q.push_back(msg.clone());
        }
        let _ = self.tx.send(StoreEvent::Message(Box::new(msg)));
    }

    /// Returns up to `limit` most-recent messages, newest first.
    pub fn list(&self, limit: usize) -> Vec<Message> {
        let q = self.inner.lock().expect("store poisoned");
        q.iter().rev().take(limit).cloned().collect()
    }

    pub fn get(&self, id: Uuid) -> Option<Message> {
        let q = self.inner.lock().expect("store poisoned");
        q.iter().find(|m| m.id == id).cloned()
    }

    /// Remove a single message. Returns `true` if it was present.
    pub fn delete(&self, id: Uuid) -> bool {
        let removed = {
            let mut q = self.inner.lock().expect("store poisoned");
            if let Some(pos) = q.iter().position(|m| m.id == id) {
                q.remove(pos).is_some()
            } else {
                false
            }
        };
        if removed {
            let _ = self.tx.send(StoreEvent::Deleted(id));
        }
        removed
    }

    pub fn clear(&self) {
        {
            let mut q = self.inner.lock().expect("store poisoned");
            q.clear();
        }
        let _ = self.tx.send(StoreEvent::Cleared);
    }

    pub fn subscribe(&self) -> broadcast::Receiver<StoreEvent> {
        self.tx.subscribe()
    }

    pub fn len(&self) -> usize {
        self.inner.lock().expect("store poisoned").len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::Bytes;

    fn msg(subject: &str) -> Message {
        crate::message::parse_message(
            Bytes::copy_from_slice(
                format!("From: a@x\r\nTo: b@x\r\nSubject: {subject}\r\n\r\nbody\r\n").as_bytes(),
            ),
            "a@x".into(),
            vec!["b@x".into()],
            "1.1.1.1:1".into(),
            false,
        )
    }

    #[test]
    fn capacity_minimum_one() {
        let s = MessageStore::new(0);
        assert_eq!(s.capacity(), 1);
    }

    #[test]
    fn push_list_get_clear() {
        let s = MessageStore::new(10);
        let m1 = msg("a");
        let m2 = msg("b");
        s.push(m1.clone());
        s.push(m2.clone());
        assert_eq!(s.len(), 2);
        let list = s.list(10);
        // newest first
        assert_eq!(list[0].id, m2.id);
        assert_eq!(list[1].id, m1.id);
        assert!(s.get(m1.id).is_some());
        assert!(s.get(Uuid::new_v4()).is_none());
        s.clear();
        assert!(s.is_empty());
    }

    #[test]
    fn delete_removes_one_and_broadcasts() {
        let s = MessageStore::new(10);
        let m = msg("only");
        s.push(m.clone());
        let mut rx = s.subscribe();
        assert!(s.delete(m.id));
        assert!(s.is_empty());
        assert!(matches!(rx.try_recv(), Ok(StoreEvent::Deleted(_)) | Err(_)));
        // Deleting again is a no-op.
        assert!(!s.delete(m.id));
    }

    #[test]
    fn evicts_oldest_when_full() {
        let s = MessageStore::new(2);
        let a = msg("a");
        let b = msg("b");
        let c = msg("c");
        s.push(a.clone());
        s.push(b.clone());
        s.push(c.clone());
        assert_eq!(s.len(), 2);
        let list = s.list(10);
        assert_eq!(list[0].id, c.id);
        assert_eq!(list[1].id, b.id);
        assert!(s.get(a.id).is_none());
    }

    #[test]
    fn list_limit_respected() {
        let s = MessageStore::new(100);
        for i in 0..50 {
            s.push(msg(&format!("m{i}")));
        }
        assert_eq!(s.list(10).len(), 10);
        assert_eq!(s.list(100).len(), 50);
    }

    #[tokio::test]
    async fn subscribers_receive_events() {
        let s = MessageStore::new(10);
        let mut rx1 = s.subscribe();
        let mut rx2 = s.subscribe();
        let m = msg("x");
        s.push(m.clone());
        match rx1.recv().await.unwrap() {
            StoreEvent::Message(got) => assert_eq!(got.id, m.id),
            _ => panic!("expected message event"),
        }
        match rx2.recv().await.unwrap() {
            StoreEvent::Message(got) => assert_eq!(got.id, m.id),
            _ => panic!("expected message event"),
        }
        s.clear();
        assert!(matches!(rx1.recv().await.unwrap(), StoreEvent::Cleared));
    }

    #[test]
    fn subscribe_with_no_listeners_does_not_panic() {
        let s = MessageStore::new(10);
        s.push(msg("x"));
        s.clear();
    }
}
