//! `TurnHandle` — a per-turn subscription to the global event stream.

use std::pin::Pin;
use std::task::{Context, Poll};

use futures::Stream;
use tokio::sync::mpsc;

use reflect_protocol::Event;

/// A handle returned by `AgentThread::submit()`. The receiver yields events
/// until the per-turn channel is closed (when the submission loop removes the
/// entry from `turn_subs`), at which point this iterator returns `None`.
///
/// `TurnHandle` implements [`futures::Stream`], which makes the full
/// `StreamExt` combinator surface (`map`, `filter`, `take_while`, …)
/// available. The inherent [`TurnHandle::next`] is kept for callers who don't
/// want to import `StreamExt`.
pub struct TurnHandle {
    rx: mpsc::Receiver<Event>,
}

impl TurnHandle {
    pub(crate) fn new(rx: mpsc::Receiver<Event>) -> Self {
        Self { rx }
    }

    /// Public constructor used by `reflect::stream::EventStream` tests
    /// (which can't reach into the crate-private `new`).
    #[doc(hidden)]
    pub fn from_receiver_for_test(rx: mpsc::Receiver<Event>) -> Self {
        Self::new(rx)
    }

    /// Await the next event. Returns `None` once the turn is finished.
    pub async fn next(&mut self) -> Option<Event> {
        self.rx.recv().await
    }
}

impl Stream for TurnHandle {
    type Item = Event;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.rx.poll_recv(cx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::StreamExt;
    use reflect_protocol::{EVENT_ID_NONE, EventMsg};

    fn make_event(text: &str) -> Event {
        Event::new(
            EVENT_ID_NONE,
            EventMsg::AgentMessageDelta(reflect_protocol::AgentMessageDelta { delta: text.into() }),
        )
    }

    #[tokio::test]
    async fn stream_impl_yields_in_order() {
        let (tx, rx) = mpsc::channel::<Event>(4);
        let mut h = TurnHandle::new(rx);
        tx.send(make_event("a")).await.unwrap();
        tx.send(make_event("b")).await.unwrap();
        drop(tx);

        let collected: Vec<String> = h
            .by_ref()
            .map(|ev| match ev.msg {
                EventMsg::AgentMessageDelta(d) => d.delta,
                _ => String::new(),
            })
            .collect()
            .await;
        assert_eq!(collected, vec!["a".to_string(), "b".to_string()]);
    }

    #[tokio::test]
    async fn stream_impl_returns_none_on_close() {
        let (tx, rx) = mpsc::channel::<Event>(1);
        let mut h = TurnHandle::new(rx);
        drop(tx);
        assert!(h.next().await.is_none());
        // Subsequent polls keep returning None.
        assert!(StreamExt::next(&mut h).await.is_none());
    }

    #[tokio::test]
    async fn inherent_next_and_stream_next_compose() {
        let (tx, rx) = mpsc::channel::<Event>(2);
        let mut h = TurnHandle::new(rx);
        tx.send(make_event("first")).await.unwrap();
        tx.send(make_event("second")).await.unwrap();
        drop(tx);

        // Inherent next() then StreamExt::next() should drain in order.
        let a = h.next().await.unwrap();
        let b = StreamExt::next(&mut h).await.unwrap();
        let c = h.next().await;
        assert!(matches!(a.msg, EventMsg::AgentMessageDelta(ref d) if d.delta == "first"));
        assert!(matches!(b.msg, EventMsg::AgentMessageDelta(ref d) if d.delta == "second"));
        assert!(c.is_none());
    }
}
