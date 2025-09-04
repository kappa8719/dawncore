use std::str::FromStr;

use reqwest::Url;

#[tokio::main]
async fn main() {
    let http = lapi::league::AuthenticatedHttp::new("SslqNkxfGH2591Oz5mkJ-g")
        .await
        .unwrap();
    let generator =
        lapi_apigen::league::League::new(Url::from_str("https://127.0.0.1:54039/").unwrap(), http);
    generator.resolve().await;
}
