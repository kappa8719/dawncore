use std::{ffi::OsStr, fs::File, io::Write, ops::Not, path::PathBuf, str::FromStr};

use clap::{Parser, Subcommand, ValueEnum};
use regex::Regex;
use reqwest::Url;
use sysinfo::Process;

#[derive(Debug, Parser)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    Generate {
        #[arg(long, value_delimiter = ',', value_enum)]
        target: Vec<GenerateTarget>,
        #[arg(long, default_value_t = GenerateFormat::Toml, value_enum)]
        format: GenerateFormat,
        #[arg(long, default_value = "lapi-generated/")]
        output: PathBuf,
        #[arg(long, default_value_t = true)]
        separate: bool,
        #[arg(long, default_value = "127.0.0.1")]
        remote_host: String,
        #[arg(long)]
        remote_port: Option<u16>,
        #[arg(long)]
        remote_token: Option<String>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, ValueEnum)]
enum GenerateTarget {
    Types,
    Functions,
    Events,
}

#[derive(Debug, Clone, ValueEnum)]
enum GenerateFormat {
    Toml,
    Json,
}

#[tokio::main]
async fn main() {
    let args = Cli::parse();

    println!("{args:?}");

    match args.command {
        Commands::Generate {
            target,
            format,
            output,
            separate,
            remote_host: remote_origin,
            remote_port,
            remote_token,
        } => {
            let (port, token) = if remote_port.is_none() || remote_token.is_none() {
                get_token_and_port_from_system().unwrap()
            } else {
                (remote_port.unwrap(), remote_token.unwrap())
            };

            if output.is_file() {
                panic!("the output path must be directory");
            }

            if !output.exists() {
                std::fs::create_dir_all(output.as_path())
                    .expect("failed to create output directory");
            }

            let host = format!("https://{origin}:{port}/", origin = remote_origin);

            let http = lapi::league::AuthenticatedHttp::new(token.as_str())
                .await
                .unwrap();
            let generator =
                lapi_apigen::league::League::new(Url::from_str(host.as_str()).unwrap(), http);
            let resolved = generator.resolve().await.unwrap();

            if target.contains(&GenerateTarget::Types) {
                if separate {
                    for (name, spec) in resolved.types {
                        let file = output.join(format!("{name}.type.toml"));
                        let mut file = File::create(file.as_path())
                            .expect("failed to open file to write type {name}");

                        let serialized = match format {
                            GenerateFormat::Toml => toml::to_string_pretty(&spec).unwrap(),
                            GenerateFormat::Json => serde_json::to_string_pretty(&spec).unwrap(),
                        };

                        file.write_all(serialized.as_bytes())
                            .expect("failed to write serialized type spec of {name} to file");
                    }
                } else {
                    let file = output.join("types.toml");
                    let mut file = File::create(file.as_path())
                        .expect("failed to open file to write type specs");

                    let serialized = match format {
                        GenerateFormat::Toml => toml::to_string_pretty(&resolved.types).unwrap(),
                        GenerateFormat::Json => {
                            serde_json::to_string_pretty(&resolved.types).unwrap()
                        }
                    };

                    file.write_all(serialized.as_bytes())
                        .expect("failed to write serialized type specs to file");
                }
            }

            if target.contains(&GenerateTarget::Functions) {
                if separate {
                    for (name, spec) in resolved.functions {
                        let file = output.join(format!("{name}.function.toml"));
                        let mut file = File::create(file.as_path())
                            .expect("failed to open file to write function {name}");

                        let serialized = match format {
                            GenerateFormat::Toml => toml::to_string_pretty(&spec).unwrap(),
                            GenerateFormat::Json => serde_json::to_string_pretty(&spec).unwrap(),
                        };

                        file.write_all(serialized.as_bytes())
                            .expect("failed to write serialized type spec of {name} to file");
                    }
                } else {
                    let file = output.join("function.toml");
                    let mut file = File::create(file.as_path())
                        .expect("failed to open file to write function specs");

                    let serialized = match format {
                        GenerateFormat::Toml => {
                            toml::to_string_pretty(&resolved.functions).unwrap()
                        }
                        GenerateFormat::Json => {
                            serde_json::to_string_pretty(&resolved.functions).unwrap()
                        }
                    };

                    file.write_all(serialized.as_bytes())
                        .expect("failed to write serialized function specs to file");
                }
            }
        }
    }

    panic!();
}

fn get_token_and_port_from_system() -> Option<(u16, String)> {
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
    get_token_and_port_from_proc(proc)
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
