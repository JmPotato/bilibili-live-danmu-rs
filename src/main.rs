mod api;
mod danmu;
mod packet;

use async_std::task;
use danmu::{CommandType, LiveDanmuStream};

const TEST_LIVE_ROOM_ID: u64 = 843771;

// TODO: implement the command control.
fn main() {
    task::block_on(async {
        let mut live_danmu_stream = LiveDanmuStream::new(TEST_LIVE_ROOM_ID);
        let cmd_rx = live_danmu_stream.connect().await;
        loop {
            let cmd_type = cmd_rx.recv().await.unwrap();
            match cmd_type {
                CommandType::Danmu(user_name, danmu_content) => {
                    println!("{} from {}", danmu_content, user_name);
                }
            }
        }
    })
}
