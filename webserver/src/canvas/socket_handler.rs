use std::{
    pin::pin,
    time::{Duration, Instant},
};

use actix_ws::AggregatedMessage;
use futures_util::{
    future::{select, Either},
    StreamExt as _,
};
use tokio::{sync::mpsc, time::interval};

use crate::{authentication::JWTUser, canvas::server::CanvasSocketServerHandle, userstore::UserId};

use super::store::{CanvasClaim, CanvasId};

/// How often heartbeat pings are sent
const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);

/// How long before lack of client response causes a timeout
const CLIENT_TIMEOUT: Duration = Duration::from_secs(10);

/// Echo text & binary messages received from the client, respond to ping messages, and monitor
/// connection health to detect network issues and free up resources.
pub async fn start_canvas_websocket_connection(
    chat_server: CanvasSocketServerHandle,
    mut session: actix_ws::Session,
    msg_stream: actix_ws::MessageStream,
    canvas_id: CanvasId,
    user: JWTUser,
    claim: CanvasClaim
) {
    let mut last_heartbeat = Instant::now();
    let mut interval = interval(HEARTBEAT_INTERVAL);

    let (message_tx, mut message_rx) = mpsc::unbounded_channel();
    chat_server.connect(
        message_tx,
        canvas_id.clone(),
        user.id.clone(),
        user.username.clone(),
        claim.r
    ).await;

    let msg_stream = msg_stream
        .max_frame_size(128 * 1024)
        .aggregate_continuations()
        .max_continuation_size(2 * 1024 * 1024);

    let mut msg_stream = pin!(msg_stream);

    let close_reason = loop {
        // most of the futures we process need to be stack-pinned to work with select()
        let tick = pin!(interval.tick());
        let msg_rx = pin!(message_rx.recv());

        // TODO: nested select is pretty gross for readability on the match
        let messages = pin!(select(msg_stream.next(), msg_rx));

        match select(messages, tick).await {
            // commands & messages received from client
            Either::Left((Either::Left((Some(Ok(msg)), _)), _)) => {
                match msg {
                    AggregatedMessage::Ping(bytes) => {
                        last_heartbeat = Instant::now();
                        session.pong(&bytes).await.unwrap();
                    }

                    AggregatedMessage::Pong(_) => {
                        last_heartbeat = Instant::now();
                    }

                    AggregatedMessage::Text(text) => {
                        process_user_socket_msg(&chat_server, &text, canvas_id.clone(), user.id.clone()).await;
                    }

                    AggregatedMessage::Binary(_bin) => {
                        println!("unexpected binary message");
                    }

                    AggregatedMessage::Close(reason) => break reason,
                }
            }

            // client WebSocket stream error
            Either::Left((Either::Left((Some(Err(err)), _)), _)) => {
                println!("{}", err);
                break None;
            }

            // client WebSocket stream ended
            Either::Left((Either::Left((None, _)), _)) => break None,

            // chat messages received from other room participants
            Either::Left((Either::Right((Some(chat_msg), _)), _)) => {
                session.text(chat_msg).await.unwrap();
            }

            // all connection's message senders were dropped
            Either::Left((Either::Right((None, _)), _)) => unreachable!(
                "all connection message senders were dropped; chat server may have panicked"
            ),

            // heartbeat internal tick
            Either::Right((_inst, _)) => {
                // if no heartbeat ping/pong received recently, close the connection
                if Instant::now().duration_since(last_heartbeat) > CLIENT_TIMEOUT {
                    println!("User {} in {canvas_id} timed out", user.id);
                    break None;
                }

                // send heartbeat ping
                let _ = session.ping(b"").await;
            }
        };
    };
    
    chat_server.disconnect(canvas_id, user.id.clone());

    // attempt to close connection gracefully
    let _ = session.close(close_reason).await;
}

async fn process_user_socket_msg(
    chat_server: &CanvasSocketServerHandle,
    // session: &mut actix_ws::Session,
    text: &str,
    canvas_id: CanvasId,
    user_id: UserId,
) {
    // strip leading and trailing whitespace (spaces, newlines, etc.)
    let msg = text.trim();

    println!("Received message: {user_id} in {canvas_id}: {msg}");

    // session.text(response).await.unwrap();
    chat_server.broadcast_event(canvas_id, user_id, msg).await;
}