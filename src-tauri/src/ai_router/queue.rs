//! Observable request queue.
//!
//! The router doesn't *block* on the queue to dispatch work (requests are
//! executed concurrently from the caller's task). The queue exists so the
//! frontend can show a live count of in-flight requests, grouped by priority,
//! and so future work (concurrency caps, batching) has a single register.

use std::collections::{BinaryHeap, HashMap};

use chrono::Utc;
use serde::Serialize;
use tokio::sync::Mutex;

use super::models::Priority;

/// A request as it sits in the queue. Ordering uses `priority` first, then a
/// monotonically increasing insertion sequence for FIFO within a priority.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct QueuedRequest {
    pub id: String,
    pub priority: Priority,
    pub enqueued_at: String,
    seq: u64,
}

impl Ord for QueuedRequest {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // Higher priority first (BinaryHeap is a max-heap).
        // Within the same priority, earlier seq wins: flip the ordering.
        self.priority
            .cmp(&other.priority)
            .then_with(|| other.seq.cmp(&self.seq))
    }
}

impl PartialOrd for QueuedRequest {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

/// A snapshot view exposed to the frontend.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct QueueStatus {
    pub total: usize,
    pub high: usize,
    pub normal: usize,
    pub low: usize,
    /// Active in-flight request ids, in dequeue order.
    pub active_ids: Vec<String>,
}

/// Async-safe priority queue with observer utilities.
///
/// Use `enqueue(...)` + `dequeue()` for the "queueing" semantics, or use
/// `begin(..)` + `finish(..)` when the caller executes in-place but wants the
/// queue to *track* in-flight work for `status()`.
pub struct PriorityQueue {
    inner: Mutex<State>,
}

struct State {
    heap: BinaryHeap<QueuedRequest>,
    next_seq: u64,
}

impl PriorityQueue {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(State {
                heap: BinaryHeap::new(),
                next_seq: 0,
            }),
        }
    }

    /// Push a request onto the heap.
    pub async fn enqueue(&self, id: String, priority: Priority) -> QueuedRequest {
        let mut state = self.inner.lock().await;
        let seq = state.next_seq;
        state.next_seq += 1;
        let item = QueuedRequest {
            id,
            priority,
            enqueued_at: Utc::now().to_rfc3339(),
            seq,
        };
        state.heap.push(item.clone());
        item
    }

    /// Pop the highest-priority request, breaking ties in FIFO order.
    pub async fn dequeue(&self) -> Option<QueuedRequest> {
        let mut state = self.inner.lock().await;
        state.heap.pop()
    }

    /// Track a request as "started" — same as [`enqueue`] for our purposes.
    pub async fn begin(&self, id: String, priority: Priority) -> QueuedRequest {
        self.enqueue(id, priority).await
    }

    /// Remove a tracked request by id, regardless of its queue position.
    /// Returns true if an entry was removed.
    pub async fn finish(&self, id: &str) -> bool {
        let mut state = self.inner.lock().await;
        let before = state.heap.len();
        let kept: BinaryHeap<QueuedRequest> = state.heap.drain().filter(|r| r.id != id).collect();
        state.heap = kept;
        state.heap.len() < before
    }

    /// Read a snapshot.
    pub async fn status(&self) -> QueueStatus {
        let state = self.inner.lock().await;
        let mut counts: HashMap<Priority, usize> = HashMap::new();
        for req in state.heap.iter() {
            *counts.entry(req.priority).or_insert(0) += 1;
        }
        // Rebuild a sorted dequeue view without mutating the real heap.
        let mut sorted: Vec<QueuedRequest> = state.heap.iter().cloned().collect();
        sorted.sort();
        sorted.reverse();
        let active_ids = sorted.into_iter().map(|q| q.id).collect();
        QueueStatus {
            total: state.heap.len(),
            high: *counts.get(&Priority::High).unwrap_or(&0),
            normal: *counts.get(&Priority::Normal).unwrap_or(&0),
            low: *counts.get(&Priority::Low).unwrap_or(&0),
            active_ids,
        }
    }
}

impl Default for PriorityQueue {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn enqueue_then_status_counts_by_priority() {
        let q = PriorityQueue::new();
        q.enqueue("a".into(), Priority::Normal).await;
        q.enqueue("b".into(), Priority::High).await;
        q.enqueue("c".into(), Priority::Low).await;
        let s = q.status().await;
        assert_eq!(s.total, 3);
        assert_eq!(s.high, 1);
        assert_eq!(s.normal, 1);
        assert_eq!(s.low, 1);
    }

    #[tokio::test]
    async fn dequeue_returns_highest_priority_first() {
        let q = PriorityQueue::new();
        q.enqueue("low".into(), Priority::Low).await;
        q.enqueue("high".into(), Priority::High).await;
        q.enqueue("normal".into(), Priority::Normal).await;
        assert_eq!(q.dequeue().await.unwrap().id, "high");
        assert_eq!(q.dequeue().await.unwrap().id, "normal");
        assert_eq!(q.dequeue().await.unwrap().id, "low");
    }

    #[tokio::test]
    async fn within_same_priority_dequeue_is_fifo() {
        let q = PriorityQueue::new();
        q.enqueue("first".into(), Priority::Normal).await;
        q.enqueue("second".into(), Priority::Normal).await;
        q.enqueue("third".into(), Priority::Normal).await;
        assert_eq!(q.dequeue().await.unwrap().id, "first");
        assert_eq!(q.dequeue().await.unwrap().id, "second");
        assert_eq!(q.dequeue().await.unwrap().id, "third");
    }

    #[tokio::test]
    async fn dequeue_on_empty_returns_none() {
        let q = PriorityQueue::new();
        assert!(q.dequeue().await.is_none());
    }

    #[tokio::test]
    async fn finish_removes_by_id() {
        let q = PriorityQueue::new();
        q.enqueue("a".into(), Priority::Normal).await;
        q.enqueue("b".into(), Priority::Normal).await;
        assert!(q.finish("a").await);
        let s = q.status().await;
        assert_eq!(s.total, 1);
        assert_eq!(s.active_ids, vec!["b".to_owned()]);
    }

    #[tokio::test]
    async fn finish_missing_id_is_noop_returning_false() {
        let q = PriorityQueue::new();
        assert!(!q.finish("ghost").await);
    }

    #[tokio::test]
    async fn active_ids_reflect_priority_order() {
        let q = PriorityQueue::new();
        q.enqueue("l".into(), Priority::Low).await;
        q.enqueue("h".into(), Priority::High).await;
        q.enqueue("n".into(), Priority::Normal).await;
        let s = q.status().await;
        assert_eq!(
            s.active_ids,
            vec!["h".to_owned(), "n".to_owned(), "l".to_owned()]
        );
    }

    #[tokio::test]
    async fn begin_and_finish_are_symmetric() {
        let q = PriorityQueue::new();
        q.begin("r1".into(), Priority::High).await;
        assert_eq!(q.status().await.total, 1);
        q.finish("r1").await;
        assert_eq!(q.status().await.total, 0);
    }
}
