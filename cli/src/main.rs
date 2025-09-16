use std::{collections::HashMap, ffi::OsStr, fs::File, io::Write, path::PathBuf, str::FromStr};

use clap::{Parser, Subcommand, ValueEnum};
use dawncore::league::{Build, EventSpec, FunctionSpec, TypeSpec};
use dawncore_apigen::league::Resolved;
use regex::Regex;
use reqwest::Url;
use serde::{Deserialize, Serialize};
use sysinfo::Process;

#[derive(Debug, Parser)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    Extract {
        #[arg(long, value_delimiter = ',', value_enum)]
        target: Vec<ExtractTarget>,
        #[arg(long, default_value_t = FileFormat::Toml, value_enum)]
        format: FileFormat,
        #[arg(long, default_value = "./extracted/")]
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
    Generate {
        #[arg(long, default_value = "dawncore-extracted/")]
        source: PathBuf,
        #[arg(long, default_value_t = true)]
        separated_source: bool,
        #[arg(long, default_value = "./generated/src")]
        output: PathBuf,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, ValueEnum)]
enum ExtractTarget {
    Build,
    Types,
    Functions,
    Events,
}

#[tokio::main]
async fn main() {
    let args = Cli::parse();

    match args.command {
        Commands::Extract { .. } => extract(args.command).await,
        Commands::Generate { .. } => generate(args.command).await,
    }
}

async fn extract(command: Commands) {
    let Commands::Extract {
        target,
        format,
        output,
        separate,
        remote_host,
        remote_port,
        remote_token,
    } = command
    else {
        unreachable!()
    };

    let (port, token) = match remote_port.is_none() || remote_token.is_none() {
        true => get_token_and_port_from_system().unwrap(),
        false => (remote_port.unwrap(), remote_token.unwrap()),
    };

    if output.is_file() {
        panic!("the output path must be directory");
    }

    if !output.exists() {
        std::fs::create_dir_all(output.as_path()).expect("failed to create output directory");
    }

    let host = format!("https://{origin}:{port}/", origin = remote_host);

    let http = dawncore::league::AuthenticatedHttp::new(
        Url::from_str(host.as_str()).unwrap(),
        token.as_str(),
    )
    .await
    .unwrap();
    let generator =
        dawncore_apigen::league::League::new(Url::from_str(host.as_str()).unwrap(), http);
    let resolved = generator.resolve().await.unwrap();

    if target.contains(&ExtractTarget::Build) {
        let file = output.join("build.toml");
        let mut file =
            File::create(file.as_path()).expect("failed to open file to write build info");

        let serialized = serializer(format, &resolved.build);
        file.write_all(serialized.as_bytes()).unwrap();
    }

    if target.contains(&ExtractTarget::Types) {
        if separate {
            for (name, spec) in resolved.types {
                let file = output.join(format!("{name}.type.toml"));
                let mut file =
                    File::create(file.as_path()).expect("failed to open file to write type {name}");

                let serialized = serializer(format, &spec);

                file.write_all(serialized.as_bytes())
                    .expect("failed to write serialized type spec of {name} to file");
            }
        } else {
            let file = output.join("types.toml");
            let mut file =
                File::create(file.as_path()).expect("failed to open file to write type specs");

            let serialized = serializer(format, &resolved.types);

            file.write_all(serialized.as_bytes())
                .expect("failed to write serialized type specs to file");
        }
    }

    if target.contains(&ExtractTarget::Functions) {
        if separate {
            for (name, spec) in resolved.functions {
                let file = output.join(format!("{name}.function.toml"));
                let mut file = File::create(file.as_path())
                    .expect("failed to open file to write function {name}");

                let serialized = serializer(format, &spec);

                file.write_all(serialized.as_bytes())
                    .expect("failed to write serialized type spec of {name} to file");
            }
        } else {
            let file = output.join("functions.toml");
            let mut file =
                File::create(file.as_path()).expect("failed to open file to write function specs");

            let serialized = serializer(format, &resolved.functions);

            file.write_all(serialized.as_bytes())
                .expect("failed to write serialized function specs to file");
        }
    }

    if target.contains(&ExtractTarget::Events) {
        if separate {
            for (name, spec) in resolved.events {
                let file = output.join(format!("{name}.event.toml"));
                let mut file = File::create(file.as_path())
                    .expect("failed to open file to write event {name}");

                let serialized = serializer(format, &spec);

                file.write_all(serialized.as_bytes())
                    .expect("failed to write serialized type spec of {name} to file");
            }
        } else {
            let file = output.join("events.toml");
            let mut file =
                File::create(file.as_path()).expect("failed to open file to write event specs");

            let serialized = serializer(format, &resolved.events);

            file.write_all(serialized.as_bytes())
                .expect("failed to write serialized event specs to file");
        }
    }
}

async fn generate(command: Commands) {
    let Commands::Generate {
        source,
        separated_source,
        output,
    } = command
    else {
        unreachable!()
    };

    let build = {
        let content = std::fs::read_to_string(source.join("build.toml")).unwrap();
        toml::from_str::<Build>(content.as_str()).unwrap()
    };

    let mut types = Vec::new();
    let mut functions = Vec::new();
    let mut events = Vec::new();

    for entry in std::fs::read_dir(source.as_path()).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        let Some(name) = path.file_stem().and_then(|v| v.to_str()) else {
            continue;
        };
        let Some(extension) = path.extension().and_then(|v| v.to_str()) else {
            continue;
        };

        let lang = match extension {
            "toml" => FileFormat::Toml,
            "json" => FileFormat::Json,
            _ => continue,
        };

        let content = std::fs::read(entry.path().as_path()).unwrap();

        if separated_source {
            let Some(target) = name.split(".").last() else {
                continue;
            };

            match target {
                "type" => types.push(deserializer(lang, &content)),
                "function" => functions.push(deserializer(lang, &content)),
                "event" => events.push(deserializer(lang, &content)),
                _ => continue,
            }
        } else {
            match name {
                "types" => {
                    let map = deserializer::<HashMap<String, TypeSpec>>(lang, content.as_ref());
                    types.extend(map.into_values());
                }
                "functions" => {
                    let map = deserializer::<HashMap<String, FunctionSpec>>(lang, content.as_ref());
                    functions.extend(map.into_values());
                }
                "events" => {
                    let map = deserializer::<HashMap<String, EventSpec>>(lang, content.as_ref());
                    events.extend(map.into_values());
                }
                _ => continue,
            };
        }
    }

    println!("loaded {} types", types.len());
    println!("loaded {} functions", functions.len());
    println!("loaded {} events", events.len());

    let mut resolved = Resolved {
        build,
        events: HashMap::new(),
        functions: HashMap::new(),
        types: HashMap::new(),
    };

    for t in types {
        resolved.types.insert(t.name.clone(), t);
    }

    for f in functions {
        resolved.functions.insert(f.name.clone(), f);
    }

    for e in events {
        resolved.events.insert(e.name.clone(), e);
    }

    std::fs::create_dir_all(output.as_path()).unwrap();
    let mut types_output = File::create(output.join("types.rs")).unwrap();
    let mut functions_output = File::create(output.join("functions.rs")).unwrap();

    dawncore_apigen::league::codegen::write_types(&mut types_output, &resolved);
    dawncore_apigen::league::codegen::write_functions(&mut functions_output, &resolved);
}

fn serializer<'de, T>(lang: FileFormat, v: &T) -> String
where
    T: Serialize,
{
    match lang {
        FileFormat::Toml => toml::to_string_pretty(v).unwrap(),
        FileFormat::Json => serde_json::to_string_pretty(v).unwrap(),
    }
}

fn deserializer<'de, T>(lang: FileFormat, slice: &'de [u8]) -> T
where
    T: Deserialize<'de>,
{
    match lang {
        FileFormat::Toml => toml::from_slice(slice).unwrap(),
        FileFormat::Json => serde_json::from_slice(slice).unwrap(),
    }
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum FileFormat {
    Toml,
    Json,
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
