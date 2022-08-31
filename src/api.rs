use std::collections::HashMap;

use reqwest::get;
use serde::{de::DeserializeOwned, Deserialize};

type Url = String;
type Params = HashMap<String, String>;

// TODO: use macro to generate `Request`, `URL` and `Params` automatically.
pub enum Request {
    LiveRoomInfo(/* Room ID */ u64),
    LiveDanmuAuthInfo(/* Real live room ID */ u64),
}

impl From<&Request> for Params {
    fn from(request: &Request) -> Self {
        let mut params = Params::new();
        match request {
            // Ref: https://github.com/SocialSisterYi/bilibili-API-collect/blob/master/live/info.md#%E8%8E%B7%E5%8F%96%E7%9B%B4%E6%92%AD%E9%97%B4%E4%BF%A1%E6%81%AF
            Request::LiveRoomInfo(room_id) => {
                params.insert("room_id".to_string(), room_id.to_string());
            }
            // Ref: https://github.com/SocialSisterYi/bilibili-API-collect/blob/master/live/message_stream.md#%E8%8E%B7%E5%8F%96%E4%BF%A1%E6%81%AF%E6%B5%81%E8%AE%A4%E8%AF%81%E7%A7%98%E9%92%A5
            Request::LiveDanmuAuthInfo(real_room_id) => {
                params.insert("id".to_string(), real_room_id.to_string());
            }
        }
        params
    }
}

const APP_KEY: &str = "27eb53fc9058f8c3";
const APP_SEC: &str = "c2ed53a74eeefe3cf99fbd01d8c9c375";

impl Request {
    // TODO: handle the Result well rather than unwrapping it all around.
    pub async fn request<R: DeserializeOwned>(&self) -> R {
        get(format!(
            "{}?{}",
            self.get_api_url(),
            self.signed_params(APP_KEY, APP_SEC)
        ))
        .await
        .unwrap()
        .json()
        .await
        .unwrap()
    }

    fn get_api_url(&self) -> Url {
        match self {
            Request::LiveRoomInfo(_room_id) => {
                "https://api.live.bilibili.com/room/v1/Room/get_info".to_string()
            }
            Request::LiveDanmuAuthInfo(_real_room_id) => {
                "https://api.live.bilibili.com/xlive/web-room/v1/index/getDanmuInfo".to_string()
            }
        }
    }

    /// Sign the request with the given `app_key` and `app_sec` and return the URL-encoded query string.
    fn signed_params(&self, app_key: &str, app_sec: &str) -> String {
        let mut params: Params = self.into();
        params.insert("appkey".to_string(), app_key.to_string());
        let mut sorted_params = params.iter().collect::<Vec<(&String, &String)>>();
        // Sort by the param key.
        sorted_params.sort_by(|a, b| a.0.cmp(b.0));
        // Hash the URL encoded params.
        let encoded_params: String = sorted_params
            .iter()
            .map(|(k, v)| {
                format!(
                    "{}={}",
                    url::form_urlencoded::byte_serialize(k.as_bytes()).collect::<String>(),
                    url::form_urlencoded::byte_serialize(v.as_bytes()).collect::<String>(),
                )
            })
            .collect::<Vec<String>>()
            .join("&");
        let sign = md5::compute(encoded_params.clone() + app_sec);
        format!("{}&sign={:x}", encoded_params, sign)
    }
}

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
pub struct Response<T> {
    code: i8,
    message: String,
    pub data: T,
}

#[derive(Deserialize, Debug)]
pub struct LiveRoomInfoResponse {
    pub uid: u64,
    pub room_id: u64,
    pub live_status: i8,
}

#[derive(Deserialize, Debug)]
pub struct LiveDanmuAuthInfoResponse {
    pub host_list: Vec<HostInfo>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct HostInfo {
    pub host: String,
    pub port: u16,
    pub wss_port: u16,
    pub ws_port: u16,
}

impl HostInfo {
    pub fn get_wss_url(&self) -> String {
        format!("wss://{}:{}/sub", self.host, self.wss_port)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sign_request() {
        assert_eq!(
            Request::LiveDanmuAuthInfo(1)
                .signed_params("1d8b6e7d45233436", "560c52ccd288fed045859ed18bffd973"),
            "appkey=1d8b6e7d45233436&id=1&sign=0f3e35d3396fc9812b2c86942b67a275"
        );
    }
}
