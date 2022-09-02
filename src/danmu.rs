use std::time::Duration;

use async_std::{
    channel::{unbounded, Receiver, Sender},
    net::TcpStream,
    sync::Mutex,
    task::{self, sleep},
};
use async_tls::client::TlsStream;
use async_tungstenite::{
    async_std::connect_async, stream::Stream, tungstenite::protocol::Message, WebSocketStream,
};
use futures::{
    stream::{SplitSink, SplitStream},
    SinkExt, StreamExt,
};
use serde_json::Value;

use crate::{
    api::{HostInfo, LiveDanmuAuthInfoResponse, LiveRoomInfoResponse, Request, Response},
    packet::Packet,
};

type StreamSender = SplitSink<WebSocketStream<Stream<TcpStream, TlsStream<TcpStream>>>, Message>;
type StreamReceiver = SplitStream<WebSocketStream<Stream<TcpStream, TlsStream<TcpStream>>>>;

#[derive(Debug, Default)]
pub struct LiveDanmuStream {
    room_id: u64,
    real_room_id: Option<u64>,
    uid: Option<u64>,
    host_list: Option<Vec<HostInfo>>,
}

impl LiveDanmuStream {
    pub fn new(room_id: u64) -> Self {
        LiveDanmuStream {
            room_id,
            ..Default::default()
        }
    }

    /// Prepare the live Danmu token and host list.
    async fn prepare(&mut self) {
        let room_info = Request::LiveRoomInfo(self.room_id)
            .request::<Response<LiveRoomInfoResponse>>()
            .await;
        self.real_room_id = Some(room_info.data.room_id);
        self.uid = Some(room_info.data.uid);
        let live_danmu_auth_info = Request::LiveDanmuAuthInfo(room_info.data.room_id)
            .request::<Response<LiveDanmuAuthInfoResponse>>()
            .await;
        self.host_list = Some(live_danmu_auth_info.data.host_list);
    }

    /// Connect to the live Danmu stream.
    pub async fn connect(&mut self) -> Receiver<Command> {
        self.prepare().await;

        let host_info = self.select_host();
        let (ws_stream, _) = connect_async(host_info.get_wss_url()).await.unwrap();
        println!("🔗 直播间弹幕服务器连接建立完成");
        let (sender, receiver) = ws_stream.split();

        self.heartbeat_loop(sender);
        println!("💗 开始发送心跳包");

        let (tx, rx) = unbounded();
        self.receiver_loop(receiver, tx);
        println!("📺 开始收取直播间消息");
        rx
    }

    fn heartbeat_loop(&self, sender: StreamSender) {
        let uid = self.uid.unwrap();
        let real_room_id = self.real_room_id.unwrap();
        let sender = Mutex::new(sender);
        task::spawn(async move {
            sender
                .lock()
                .await
                .send(Packet::new_auth_packet(uid, real_room_id).into())
                .await
                .unwrap();
            loop {
                sender
                    .lock()
                    .await
                    .send(Packet::new_heartbeat_packet().into())
                    .await
                    .unwrap();
                sleep(Duration::from_secs(30)).await;
            }
        });
    }

    fn receiver_loop(&self, receiver: StreamReceiver, tx: Sender<Command>) {
        let receiver = Mutex::new(receiver);
        task::spawn(async move {
            loop {
                if let Message::Binary(bytes) = receiver.lock().await.next().await.unwrap().unwrap()
                {
                    let packet = Packet::from_bytes(&bytes);
                    let cmd_json_vec = packet.get_command_json();
                    if cmd_json_vec.is_none() {
                        continue;
                    }
                    for cmd_json in cmd_json_vec.unwrap() {
                        let cmd_value: Value = serde_json::from_str(cmd_json.as_str()).unwrap();
                        let cmd = match cmd_value["cmd"].as_str().unwrap() {
                            "DANMU_MSG" => Command::Danmu(
                                cmd_value["info"][2][1].as_str().unwrap().to_string(),
                                cmd_value["info"][1].as_str().unwrap().to_string(),
                            ),
                            "INTERACT_WORD" => Command::InteractWord(
                                cmd_value["data"]["uname"].as_str().unwrap().to_string(),
                            ),
                            "SEND_GIFT" => Command::SendGift(
                                cmd_value["data"]["uname"].as_str().unwrap().to_string(),
                                cmd_value["data"]["action"].as_str().unwrap().to_string(),
                                cmd_value["data"]["giftName"].as_str().unwrap().to_string(),
                                cmd_value["data"]["num"].as_u64().unwrap(),
                            ),
                            _ => {
                                continue;
                            }
                        };
                        tx.send(cmd).await.unwrap();
                    }
                };
            }
        });
    }

    // TODO: select the best host according to the network latency.
    fn select_host(&self) -> HostInfo {
        self.host_list.as_ref().unwrap().last().unwrap().clone()
    }
}

// TODO: support more command types.
pub enum Command {
    Danmu(/* Username */ String, /* Danmu content */ String),
    InteractWord(/* Username */ String),
    SendGift(
        /* Username */ String,
        /* Action */ String,
        /* Gift name */ String,
        /* Gift count */ u64,
    ),
}
