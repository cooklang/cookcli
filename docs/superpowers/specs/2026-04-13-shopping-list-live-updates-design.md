# Shopping List Live Updates — Design

## Goal

When the shopping list page (`/shopping_list`) is open and the
underlying `.shopping-list` or `.shopping-checked` files change — for
any reason (sync from another device, manual text-editor edit, mutation
from another browser tab, or same-tab API write) — the UI reflects the
new state without the user reloading the page.

## Non-goals

- Updating other pages that might show shopping-list-derived state
  (recipe detail badges, nav counts, etc.). The mechanism should be
  reusable for those later, but this change wires live updates only
  into the shopping list page itself.
- Client→client messaging, presence, or conflict resolution beyond what
  the on-disk files already encode.
- Running the watcher outside `cook server` (no CLI integration).

## Architecture

### Server

1. On `cook server` startup, spawn a single filesystem watcher over
   `base_path` using the `notify` crate (wrapped in
   `notify-debouncer-full`).
2. The debouncer emits coalesced events on a ~200 ms window. The
   handler filters to changes affecting `.shopping-list` or
   `.shopping-checked` within `base_path` (ignore anything else —
   recipe files, temp files, backups).
3. Each relevant event is pushed onto a `tokio::sync::broadcast::Sender`
   stored on `AppState`. Payload:
   `ShoppingListChangeEvent { file: "list" | "checked" }`.
4. A new route `GET /api/shopping_list/events` returns an
   `axum::response::sse::Sse` stream. Each connection calls
   `broadcast::Sender::subscribe()` and forwards events as SSE
   messages. On `Lagged`, the receiver resumes without closing the
   stream (the client will catch up on its next fetch).
5. Watcher init failure is logged at WARN and the server keeps running
   without live updates. The SSE endpoint still works but never emits;
   that's acceptable degradation.

### Client

1. The existing `/shopping_list` page gains an `EventSource` pointed at
   `{prefix}/api/shopping_list/events`, opened after the initial load.
2. On any `message` event, the page calls the existing load function
   (the one already wired to page load / after-mutation refresh) —
   reusing whatever assembles the categorized view from
   `/api/shopping_list/items`, `/api/shopping_list`, and
   `/api/shopping_list/checked`.
3. `EventSource` handles reconnection natively; no custom retry logic.
4. The stream is closed on `beforeunload` to be a good citizen.

## Data flow

```
disk write  ─▶  notify  ─▶  debouncer  ─▶  broadcast::Sender
                                                 │
                                                 ▼ subscribe()
                                          per-connection SSE task
                                                 │
                                                 ▼ text/event-stream
                                          EventSource (browser)
                                                 │
                                                 ▼
                                          re-fetch + re-render
```

## Key choices

- **SSE over WebSocket.** One-way server→client suffices; SSE has
  native auto-reconnect and a trivial server implementation in axum.
- **Broadcast channel with a single watcher.** One watcher process-wide,
  N subscribers. Future non-shopping-list consumers can subscribe to
  the same channel (or a sibling) cheaply.
- **Ping, not payload.** The event carries only which file changed.
  The client re-uses the existing fetch + render pipeline. This keeps
  the SSE channel decoupled from the richer categorized response
  (which depends on aisle/pantry config and ingredient aggregation).
- **Accept self-echo.** A same-tab API write causes the watcher to
  fire, and the tab re-fetches its own write. Result is identical, so
  no suppression logic is worth adding.
- **`notify-debouncer-full` window = 200 ms.** Compaction does an
  atomic rename that emits create+modify; debouncing collapses this to
  one event.

## Error handling

| Scenario | Behavior |
|---|---|
| `notify` init fails on startup | WARN log; server runs without live updates. |
| Watcher sees a non-target path | Ignored by filter. |
| Broadcast has no receivers | Send returns `Err`; watcher task discards and continues. |
| Receiver lags (slow client) | `Lagged(n)` variant handled: log at DEBUG, continue. |
| Client disconnects | Tokio task ends when SSE stream is dropped. |
| File doesn't exist yet (first-time setup) | Watcher watches the directory, so new file creation fires an event; no special-casing needed. |

## Testing

- **Rust unit:** the debouncer filter — feed mixed paths, assert only
  `.shopping-list` / `.shopping-checked` events pass through.
- **Rust integration:** spawn the server against a tempdir, open the
  SSE stream with `reqwest`, write to `.shopping-list` on disk, assert
  exactly one event arrives within a short timeout. Repeat rapidly to
  confirm debouncing coalesces.
- **Playwright e2e:** on the shopping list page, write to
  `.shopping-list` out-of-band (via a test fixture), assert the
  rendered list updates without a reload. Add a second tab in the same
  test to confirm cross-tab updates.

## Out of scope / future work

- Site-wide subscription for recipe pages, nav counts, etc. — the
  broadcast channel is ready; additional consumers are incremental.
- Payload-inclusive events (send the new list in the SSE message to
  skip the re-fetch). Only worth it if `/api/shopping_list` becomes a
  bottleneck.
