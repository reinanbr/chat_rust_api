use warp::Filter;
use tokio::sync::broadcast;
use warp::ws::{Message, WebSocket};
use futures_util::{StreamExt, SinkExt};
use serde::{Serialize, Deserialize};
use serde_json;

#[derive(Serialize, Deserialize, Clone, Debug)]
struct ChatMessage {
    username: String,
    message: String,
}

#[tokio::main]
async fn main() {
    let (tx, _rx) = broadcast::channel::<String>(10);

    let chat = warp::path("ws")
        .and(warp::ws())
        .and(warp::any().map(move || tx.clone()))
        .map(|ws: warp::ws::Ws, tx| ws.on_upgrade(move |socket| handle_socket(socket, tx)));

    let routes = chat;
    warp::serve(routes).run(([127, 0, 0, 1], 3030)).await;
}

async fn handle_socket(ws: WebSocket, tx: broadcast::Sender<String>) {
    let (mut sender, mut receiver) = ws.split();
    let mut rx = tx.subscribe();

    // Nome do usuário (pode ser enviado pelo cliente na conexão inicial)
    let username = "User".to_string(); // Pode ser ajustado para receber do cliente.

    // Envia mensagem de "entrada" para todos
    let entry_msg = ChatMessage {
        username: username.clone(),
        message: "has joined the chat".to_string(),
    };
    let entry_msg_json = serde_json::to_string(&entry_msg).unwrap();
    tx.send(entry_msg_json).ok();

    // Lida com a recepção de mensagens e envio de broadcast
    tokio::spawn(async move {
        while let Ok(msg) = rx.recv().await {
            sender.send(Message::text(msg)).await.ok();
        }
    });

    // Lê as mensagens recebidas dos clientes
    while let Some(Ok(msg)) = receiver.next().await {
        if let Ok(text) = msg.to_str() {
            if let Ok(chat_msg) = serde_json::from_str::<ChatMessage>(text) {
                println!("Received message from {}: {}", chat_msg.username, chat_msg.message);
                // Transmite a mensagem para todos
                let broadcast_message = serde_json::to_string(&chat_msg).unwrap();
                tx.send(broadcast_message).ok();
            }
        }
    }

    // Envia mensagem de "saída" quando o cliente desconectar
    let exit_msg = ChatMessage {
        username: username.clone(),
        message: "has left the chat".to_string(),
    };
    let exit_msg_json = serde_json::to_string(&exit_msg).unwrap();
    tx.send(exit_msg_json).ok();
}
