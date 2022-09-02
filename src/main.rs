mod api;
mod danmu;
mod packet;

use async_std::task;
use clap::Parser;
use colored::Colorize;
use danmu::{Command, LiveDanmuStream};

#[derive(Parser)]
#[clap(name = "bilibili-live-danmu-ctl")]
#[clap(author, version, about)] // Read from `Cargo.toml`
struct Cli {
    #[clap(short = 'r', long = "room", value_parser)]
    room_id: u64,
}

// TODO: implement the command control.
fn main() {
    let cli = Cli::parse();

    task::block_on(async {
        let mut live_danmu_stream = LiveDanmuStream::new(cli.room_id);
        let cmd_rx = live_danmu_stream.connect().await;
        loop {
            match cmd_rx.recv().await.unwrap() {
                Command::Danmu(username, danmu_content) => {
                    println!("{}: {}", username.bold(), danmu_content);
                }
                Command::InteractWord(username) => {
                    println!("{} {}", username.bold(), "进入了直播间".italic().yellow());
                }
                Command::SendGift(username, action, gift_name, gift_number) => {
                    println!(
                        "{} {}了 {} x {}",
                        username.bold(),
                        action,
                        gift_name.red().bold(),
                        gift_number
                    );
                }
            }
        }
    })
}
