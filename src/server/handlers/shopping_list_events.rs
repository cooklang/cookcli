//! SSE endpoint that streams `.shopping-list` / `.shopping-checked` change
//! pings to the browser. Each connected client re-fetches the shopping
//! list data via the existing JSON endpoints on every event.

use crate::server::shopping_list_watcher::ShoppingListChangeEvent;
use crate::server::AppState;
use axum::{
    extract::State,
    response::sse::{Event, KeepAlive, Sse},
};
use futures_util::stream::{self, Stream};
use std::{convert::Infallible, sync::Arc, time::Duration};
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::StreamExt;

pub async fn shopping_list_events(
    State(state): State<Arc<AppState>>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let stream: Box<dyn Stream<Item = Result<Event, Infallible>> + Send + Unpin> =
        match &state.shopping_list_events {
            Some(tx) => {
                let rx = tx.subscribe();
                // Lagged receivers become empty events (client will re-fetch
                // anyway). Channel close would end the stream, but the
                // sender is held by AppState for the life of the process.
                let s = BroadcastStream::new(rx).filter_map(
                    |res: Result<ShoppingListChangeEvent, _>| match res {
                        Ok(evt) => Some(Ok(Event::default()
                            .event("change")
                            .json_data(evt)
                            .unwrap_or_else(|_| Event::default().event("change")))),
                        Err(_lagged) => {
                            tracing::debug!("SSE subscriber lagged — client will catch up on next fetch");
                            None
                        }
                    },
                );
                Box::new(s)
            }
            None => {
                // Watcher failed to init; serve a well-formed but silent stream.
                Box::new(stream::pending::<Result<Event, Infallible>>())
            }
        };

    Sse::new(stream).keep_alive(
        KeepAlive::new()
            .interval(Duration::from_secs(30))
            .text("ping"),
    )
}
