use std::{ffi::OsStr, str::FromStr};

use regex::Regex;
use reqwest::Url;
use sysinfo::Process;

#[tokio::main]
async fn main() {
    let system = sysinfo::System::new_all();
    let proc = system
        .processes()
        .values()
        .find(|proc| {
            let Some(name) = proc.name().to_str() else {
                return false;
            };

            matches!(name, "LeagueClientUx.exe" | "LeagueClientUx")
        })
        .expect("failed to find running LeagueClientUx(.exe)");
    let Some((port, token)) = get_token_and_port_from_proc(proc) else {
        return;
    };

    let http = lapi::league::AuthenticatedHttp::new(token.as_str())
        .await
        .unwrap();
    let generator = lapi_apigen::league::League::new(
        Url::from_str(format!("https://127.0.0.1:{port}/").as_str()).unwrap(),
        http,
    );
    generator.resolve().await;
}

fn get_token_and_port_from_proc(proc: &Process) -> Option<(u16, String)> {
    let regex_port = Regex::new("--app-port=([0-9]*)").unwrap();
    let regex_token = Regex::new("--remoting-auth-token=([\\w-]*)").unwrap();

    let command = proc.cmd().join(OsStr::new(" ")).into_string().unwrap();
    let port = regex_port
        .captures(command.as_str())
        .and_then(|v| v.get(1))
        .and_then(|v| v.as_str().parse::<u16>().ok());
    let token = regex_token
        .captures(command.as_str())
        .and_then(|v| v.get(1))
        .map(|v| v.as_str().to_string());

    Some((port?, token?))
}
