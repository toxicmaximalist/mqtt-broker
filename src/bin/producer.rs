//! MQTT Producer (Publisher) CLI binary.
//!
//! A simple command-line MQTT client for publishing messages.
//!
//! Usage: mqtt-producer [OPTIONS] <TOPIC> <MESSAGE>
//!
//! Arguments:
//!   <TOPIC>     Topic to publish to
//!   <MESSAGE>   Message payload to publish
//!
//! Options:
//!   -h, --host <HOST>    Broker host address [default: 127.0.0.1]
//!   -p, --port <PORT>    Broker port [default: 1883]
//!   -q, --qos <QOS>      Quality of Service level (0, 1, 2) [default: 0]
//!   -r, --retain         Retain the message
//!   -c, --client <ID>    Client ID [default: auto-generated]
//!   --help               Print help

use std::env;
use std::net::SocketAddr;
use std::process::exit;
use std::time::Duration;

use bytes::Bytes;
use tokio::net::TcpStream;

use mqtt_broker::codec::{
    Connect, Publish, QoS, Packet,
};
use mqtt_broker::transport::{Connection, ConnectionConfig};

struct Args {
    host: String,
    port: u16,
    topic: String,
    message: String,
    qos: QoS,
    retain: bool,
    client_id: Option<String>,
}

fn print_help() {
    eprintln!("MQTT Producer - Publish messages to an MQTT broker");
    eprintln!();
    eprintln!("Usage: mqtt-producer [OPTIONS] <TOPIC> <MESSAGE>");
    eprintln!();
    eprintln!("Arguments:");
    eprintln!("  <TOPIC>     Topic to publish to");
    eprintln!("  <MESSAGE>   Message payload to publish");
    eprintln!();
    eprintln!("Options:");
    eprintln!("  -h, --host <HOST>    Broker host address [default: 127.0.0.1]");
    eprintln!("  -p, --port <PORT>    Broker port [default: 1883]");
    eprintln!("  -q, --qos <QOS>      Quality of Service level (0, 1, 2) [default: 0]");
    eprintln!("  -r, --retain         Retain the message");
    eprintln!("  -c, --client <ID>    Client ID [default: auto-generated]");
    eprintln!("  --help               Print help");
}

fn parse_args() -> Result<Args, String> {
    let args: Vec<String> = env::args().collect();
    
    let mut host = "127.0.0.1".to_string();
    let mut port: u16 = 1883;
    let mut qos = QoS::AtMostOnce;
    let mut retain = false;
    let mut client_id: Option<String> = None;
    let mut positional: Vec<String> = Vec::new();
    
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
            "-q" | "--qos" => {
                if i + 1 >= args.len() {
                    return Err("Missing value for --qos".to_string());
                }
                qos = match args[i + 1].as_str() {
                    "0" => QoS::AtMostOnce,
                    "1" => QoS::AtLeastOnce,
                    "2" => QoS::ExactlyOnce,
                    _ => return Err(format!("Invalid QoS: {} (must be 0, 1, or 2)", args[i + 1])),
                };
                i += 2;
            }
            "-r" | "--retain" => {
                retain = true;
                i += 1;
            }
            "-c" | "--client" => {
                if i + 1 >= args.len() {
                    return Err("Missing value for --client".to_string());
                }
                client_id = Some(args[i + 1].clone());
                i += 2;
            }
            arg if arg.starts_with('-') => {
                return Err(format!("Unknown argument: {}", arg));
            }
            _ => {
                positional.push(args[i].clone());
                i += 1;
            }
        }
    }
    
    if positional.len() < 2 {
        return Err("Missing required arguments: <TOPIC> <MESSAGE>".to_string());
    }
    
    Ok(Args {
        host,
        port,
        topic: positional[0].clone(),
        message: positional[1..].join(" "),
        qos,
        retain,
        client_id,
    })
}

#[tokio::main]
async fn main() {
    let args = match parse_args() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("Error: {}", e);
            eprintln!();
            print_help();
            exit(1);
        }
    };
    
    let addr: SocketAddr = format!("{}:{}", args.host, args.port).parse()
        .expect("Invalid address");
    
    // Connect to broker
    println!("Connecting to {}...", addr);
    
    let stream = match TcpStream::connect(addr).await {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Failed to connect: {}", e);
            exit(1);
        }
    };
    
    let mut conn = Connection::new(stream, ConnectionConfig::default())
        .expect("Failed to create connection");
    
    // Generate client ID
    let client_id = args.client_id.unwrap_or_else(|| {
        format!("mqtt-producer-{}", std::process::id())
    });
    
    // Send CONNECT
    let connect = Packet::Connect(Connect::new(client_id.clone())
        .clean_session(true)
        .keep_alive(60));
    
    conn.send_packet(&connect).await.expect("Failed to send CONNECT");
    
    // Wait for CONNACK
    let connack = tokio::time::timeout(
        Duration::from_secs(5),
        conn.read_packet()
    ).await;
    
    match connack {
        Ok(Ok(Some(Packet::ConnAck(ack)))) => {
            if !ack.return_code.is_accepted() {
                eprintln!("Connection rejected: {:?}", ack.return_code);
                exit(1);
            }
            println!("Connected as '{}'", client_id);
        }
        Ok(Ok(Some(_))) => {
            eprintln!("Unexpected packet, expected CONNACK");
            exit(1);
        }
        Ok(Ok(None)) => {
            eprintln!("Connection closed by broker");
            exit(1);
        }
        Ok(Err(e)) => {
            eprintln!("Error reading CONNACK: {}", e);
            exit(1);
        }
        Err(_) => {
            eprintln!("Timeout waiting for CONNACK");
            exit(1);
        }
    }
    
    // Build PUBLISH packet
    let publish = if args.qos == QoS::AtMostOnce {
        Publish::new(&args.topic, Bytes::from(args.message.clone()))
            .retain(args.retain)
    } else {
        Publish::with_qos(&args.topic, Bytes::from(args.message.clone()), args.qos, 1)
            .retain(args.retain)
    };
    
    // Send PUBLISH
    conn.send_packet(&Packet::Publish(publish)).await.expect("Failed to send PUBLISH");
    
    println!("Published to '{}': {}", args.topic, args.message);
    
    // Handle QoS acknowledgments
    if args.qos == QoS::AtLeastOnce {
        // Wait for PUBACK
        match tokio::time::timeout(Duration::from_secs(5), conn.read_packet()).await {
            Ok(Ok(Some(Packet::PubAck(_)))) => {
                println!("Message acknowledged (QoS 1)");
            }
            _ => {
                eprintln!("Warning: No acknowledgment received");
            }
        }
    } else if args.qos == QoS::ExactlyOnce {
        // Wait for PUBREC
        match tokio::time::timeout(Duration::from_secs(5), conn.read_packet()).await {
            Ok(Ok(Some(Packet::PubRec(pubrec)))) => {
                // Send PUBREL
                conn.send_packet(&Packet::PubRel(mqtt_broker::codec::PubRel { 
                    packet_id: pubrec.packet_id 
                })).await.expect("Failed to send PUBREL");
                
                // Wait for PUBCOMP
                match tokio::time::timeout(Duration::from_secs(5), conn.read_packet()).await {
                    Ok(Ok(Some(Packet::PubComp(_)))) => {
                        println!("Message delivered exactly once (QoS 2)");
                    }
                    _ => {
                        eprintln!("Warning: QoS 2 flow incomplete");
                    }
                }
            }
            _ => {
                eprintln!("Warning: No PUBREC received");
            }
        }
    }
    
    // Send DISCONNECT
    conn.send_packet(&Packet::Disconnect).await.ok();
    
    println!("Done.");
}