//! MQTT Consumer (Subscriber) CLI binary.
//!
//! A simple command-line MQTT client for subscribing to topics.
//!
//! Usage: mqtt-consumer [OPTIONS] <TOPIC>...
//!
//! Arguments:
//!   <TOPIC>...    Topic filter(s) to subscribe to (supports wildcards: +, #)
//!
//! Options:
//!   -h, --host <HOST>    Broker host address [default: 127.0.0.1]
//!   -p, --port <PORT>    Broker port [default: 1883]
//!   -q, --qos <QOS>      Maximum QoS level (0, 1, 2) [default: 0]
//!   -c, --client <ID>    Client ID [default: auto-generated]
//!   -n, --count <N>      Exit after receiving N messages [default: unlimited]
//!   --help               Print help

use std::env;
use std::net::SocketAddr;
use std::process::exit;
use std::time::Duration;

use tokio::net::TcpStream;

use mqtt_broker::codec::{
    Connect, Subscribe, Subscription, QoS, Packet,
};
use mqtt_broker::transport::{Connection, ConnectionConfig};

struct Args {
    host: String,
    port: u16,
    topics: Vec<String>,
    qos: QoS,
    client_id: Option<String>,
    count: Option<usize>,
}

fn print_help() {
    eprintln!("MQTT Consumer - Subscribe to topics on an MQTT broker");
    eprintln!();
    eprintln!("Usage: mqtt-consumer [OPTIONS] <TOPIC>...");
    eprintln!();
    eprintln!("Arguments:");
    eprintln!("  <TOPIC>...    Topic filter(s) to subscribe to (supports wildcards: +, #)");
    eprintln!();
    eprintln!("Options:");
    eprintln!("  -h, --host <HOST>    Broker host address [default: 127.0.0.1]");
    eprintln!("  -p, --port <PORT>    Broker port [default: 1883]");
    eprintln!("  -q, --qos <QOS>      Maximum QoS level (0, 1, 2) [default: 0]");
    eprintln!("  -c, --client <ID>    Client ID [default: auto-generated]");
    eprintln!("  -n, --count <N>      Exit after receiving N messages [default: unlimited]");
    eprintln!("  --help               Print help");
    eprintln!();
    eprintln!("Examples:");
    eprintln!("  mqtt-consumer sensor/temperature");
    eprintln!("  mqtt-consumer 'sensor/+/temperature' -q 1");
    eprintln!("  mqtt-consumer 'home/#' -n 10");
}

fn parse_args() -> Result<Args, String> {
    let args: Vec<String> = env::args().collect();
    
    let mut host = "127.0.0.1".to_string();
    let mut port: u16 = 1883;
    let mut qos = QoS::AtMostOnce;
    let mut client_id: Option<String> = None;
    let mut count: Option<usize> = None;
    let mut topics: Vec<String> = Vec::new();
    
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
            "-c" | "--client" => {
                if i + 1 >= args.len() {
                    return Err("Missing value for --client".to_string());
                }
                client_id = Some(args[i + 1].clone());
                i += 2;
            }
            "-n" | "--count" => {
                if i + 1 >= args.len() {
                    return Err("Missing value for --count".to_string());
                }
                count = Some(args[i + 1].parse()
                    .map_err(|_| format!("Invalid count: {}", args[i + 1]))?);
                i += 2;
            }
            arg if arg.starts_with('-') => {
                return Err(format!("Unknown argument: {}", arg));
            }
            _ => {
                topics.push(args[i].clone());
                i += 1;
            }
        }
    }
    
    if topics.is_empty() {
        return Err("At least one topic filter is required".to_string());
    }
    
    Ok(Args {
        host,
        port,
        topics,
        qos,
        client_id,
        count,
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
        format!("mqtt-consumer-{}", std::process::id())
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
    
    // Build SUBSCRIBE packet
    let subscriptions: Vec<Subscription> = args.topics.iter()
        .map(|t| Subscription {
            topic_filter: t.clone(),
            qos: args.qos,
        })
        .collect();
    
    let subscribe = Packet::Subscribe(Subscribe {
        packet_id: 1,
        subscriptions,
    });
    
    conn.send_packet(&subscribe).await.expect("Failed to send SUBSCRIBE");
    
    // Wait for SUBACK
    match tokio::time::timeout(Duration::from_secs(5), conn.read_packet()).await {
        Ok(Ok(Some(Packet::SubAck(suback)))) => {
            println!("Subscribed to {} topic(s):", args.topics.len());
            for (topic, code) in args.topics.iter().zip(suback.return_codes.iter()) {
                println!("  {} -> {:?}", topic, code);
            }
        }
        Ok(Ok(Some(_))) => {
            eprintln!("Unexpected packet, expected SUBACK");
            exit(1);
        }
        Ok(Ok(None)) | Ok(Err(_)) | Err(_) => {
            eprintln!("Failed to receive SUBACK");
            exit(1);
        }
    }
    
    println!();
    println!("Waiting for messages... (Press Ctrl+C to exit)");
    println!();
    
    // Message receive loop
    let mut message_count = 0usize;
    
    loop {
        tokio::select! {
            result = conn.read_packet() => {
                match result {
                    Ok(Some(Packet::Publish(publish))) => {
                        message_count += 1;
                        
                        // Display the message
                        let payload = String::from_utf8_lossy(&publish.payload);
                        println!("[{}] Topic: {} | QoS: {:?} | Retain: {}",
                            message_count,
                            publish.topic,
                            publish.qos,
                            publish.retain
                        );
                        println!("    Payload: {}", payload);
                        println!();
                        
                        // Handle QoS acknowledgments
                        if let Some(packet_id) = publish.packet_id {
                            match publish.qos {
                                QoS::AtLeastOnce => {
                                    // Send PUBACK
                                    conn.send_packet(&Packet::PubAck(mqtt_broker::codec::PubAck { 
                                        packet_id 
                                    })).await.ok();
                                }
                                QoS::ExactlyOnce => {
                                    // Send PUBREC
                                    conn.send_packet(&Packet::PubRec(mqtt_broker::codec::PubRec { 
                                        packet_id 
                                    })).await.ok();
                                }
                                QoS::AtMostOnce => {}
                            }
                        }
                        
                        // Check if we've received enough messages
                        if let Some(max) = args.count {
                            if message_count >= max {
                                println!("Received {} message(s), exiting.", max);
                                break;
                            }
                        }
                    }
                    Ok(Some(Packet::PubRel(pubrel))) => {
                        // Complete QoS 2 flow
                        conn.send_packet(&Packet::PubComp(mqtt_broker::codec::PubComp { 
                            packet_id: pubrel.packet_id 
                        })).await.ok();
                    }
                    Ok(Some(Packet::PingResp)) => {
                        // Keep-alive response, ignore
                    }
                    Ok(Some(_)) => {
                        // Ignore other packets
                    }
                    Ok(None) => {
                        println!("Connection closed by broker");
                        break;
                    }
                    Err(e) => {
                        eprintln!("Error: {}", e);
                        break;
                    }
                }
            }
            _ = tokio::signal::ctrl_c() => {
                println!("\nReceived Ctrl+C, disconnecting...");
                break;
            }
        }
    }
    
    // Send DISCONNECT
    conn.send_packet(&Packet::Disconnect).await.ok();
    
    println!("Total messages received: {}", message_count);
}