//! MQTT Broker binary entry point.
//!
//! Usage: mqtt-broker [OPTIONS]
//!
//! Options:
//!   -h, --host <HOST>    Host address to bind to [default: 0.0.0.0]
//!   -p, --port <PORT>    Port to listen on [default: 1883]
//!   --max-clients <N>    Maximum concurrent clients [default: 10000]
//!   --help               Print help

use std::env;
use std::net::SocketAddr;
use std::process::exit;

use mqtt_broker::server::{Server, ServerConfig};

fn print_help() {
    eprintln!("MQTT Broker - Production-grade MQTT 3.1.1 broker");
    eprintln!();
    eprintln!("Usage: mqtt-broker [OPTIONS]");
    eprintln!();
    eprintln!("Options:");
    eprintln!("  -h, --host <HOST>    Host address to bind to [default: 0.0.0.0]");
    eprintln!("  -p, --port <PORT>    Port to listen on [default: 1883]");
    eprintln!("  --max-clients <N>    Maximum concurrent clients [default: 10000]");
    eprintln!("  --help               Print help");
}

fn parse_args() -> Result<ServerConfig, String> {
    let args: Vec<String> = env::args().collect();
    
    let mut host = "0.0.0.0".to_string();
    let mut port: u16 = 1883;
    let mut max_clients: usize = 10000;
    
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--help" => {
                print_help();
                exit(0);
            }
            "-h" | "--host" => {
                if i + 1 >= args.len() {
                    return Err("Missing value for --host".to_string());
                }
                host = args[i + 1].clone();
                i += 2;
            }
            "-p" | "--port" => {
                if i + 1 >= args.len() {
                    return Err("Missing value for --port".to_string());
                }
                port = args[i + 1].parse()
                    .map_err(|_| format!("Invalid port: {}", args[i + 1]))?;
                i += 2;
            }
            "--max-clients" => {
                if i + 1 >= args.len() {
                    return Err("Missing value for --max-clients".to_string());
                }
                max_clients = args[i + 1].parse()
                    .map_err(|_| format!("Invalid max-clients: {}", args[i + 1]))?;
                i += 2;
            }
            arg => {
                return Err(format!("Unknown argument: {}", arg));
            }
        }
    }
    
    let addr: SocketAddr = format!("{}:{}", host, port).parse()
        .map_err(|_| format!("Invalid address: {}:{}", host, port))?;
    
    let mut config = ServerConfig::new(addr);
    config.max_connections = max_clients;
    
    Ok(config)
}

#[tokio::main]
async fn main() {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into())
        )
        .init();
    
    // Parse arguments
    let config = match parse_args() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Error: {}", e);
            eprintln!();
            print_help();
            exit(1);
        }
    };
    
    let bind_addr = config.bind_addr;
    
    // Create and run server
    let server = Server::new(config);
    
    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║              MQTT Broker v0.1.0 Starting                     ║");
    println!("╠══════════════════════════════════════════════════════════════╣");
    println!("║  Protocol: MQTT 3.1.1                                        ║");
    println!("║  Address:  {:<50} ║", bind_addr);
    println!("╚══════════════════════════════════════════════════════════════╝");
    println!();
    
    // Handle Ctrl+C for graceful shutdown
    let _server_ref = server.router();
    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.expect("Failed to listen for Ctrl+C");
        tracing::info!("Received Ctrl+C, shutting down...");
    });
    
    // Run the server
    if let Err(e) = server.run().await {
        eprintln!("Server error: {}", e);
        exit(1);
    }
    
    println!("Server shutdown complete.");
}