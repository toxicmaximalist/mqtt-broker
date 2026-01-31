//! Packet handler for processing MQTT packets.

use std::sync::Arc;
use parking_lot::RwLock;

use crate::codec::{
    Packet, Connect, ConnAck, Publish, PubAck, PubRec, PubRel, PubComp,
    Subscribe, SubAck, Unsubscribe, UnsubAck,
    QoS, ConnectReturnCode, SubAckCode,
};
use crate::router::MessageRouter;
use super::client::Client;

/// Result of handling a packet.
#[derive(Debug)]
pub enum HandleResult {
    /// Send a response packet.
    Response(Packet),
    /// Send multiple response packets.
    Responses(Vec<Packet>),
    /// No response needed.
    None,
    /// Disconnect the client.
    Disconnect,
    /// Error occurred - disconnect with optional reason.
    Error(String),
}

/// Packet handler for processing incoming MQTT packets.
pub struct PacketHandler {
    /// The message router.
    router: Arc<RwLock<MessageRouter>>,
}

impl PacketHandler {
    /// Create a new packet handler.
    pub fn new(router: Arc<RwLock<MessageRouter>>) -> Self {
        Self { router }
    }

    /// Handle an incoming packet.
    pub fn handle(&self, client: &mut Client, packet: Packet) -> HandleResult {
        // Update activity timestamp
        client.touch();

        match packet {
            Packet::Connect(connect) => self.handle_connect(client, connect),
            Packet::Publish(publish) => self.handle_publish(client, publish),
            Packet::PubAck(puback) => self.handle_puback(client, puback),
            Packet::PubRec(pubrec) => self.handle_pubrec(client, pubrec),
            Packet::PubRel(pubrel) => self.handle_pubrel(client, pubrel),
            Packet::PubComp(pubcomp) => self.handle_pubcomp(client, pubcomp),
            Packet::Subscribe(subscribe) => self.handle_subscribe(client, subscribe),
            Packet::Unsubscribe(unsubscribe) => self.handle_unsubscribe(client, unsubscribe),
            Packet::PingReq => self.handle_pingreq(client),
            Packet::Disconnect => self.handle_disconnect(client),
            
            // Server should not receive these:
            Packet::ConnAck(_) | Packet::SubAck(_) | Packet::UnsubAck(_) | Packet::PingResp => {
                HandleResult::Error("Unexpected packet type from client".to_string())
            }
        }
    }

    /// Handle CONNECT packet.
    fn handle_connect(&self, client: &mut Client, connect: Connect) -> HandleResult {
        // Must be in Connecting state
        if !client.is_connecting() {
            return HandleResult::Error("CONNECT received in wrong state".to_string());
        }

        // Validate protocol
        if connect.protocol_name != "MQTT" || connect.protocol_level != 4 {
            return HandleResult::Response(Packet::ConnAck(ConnAck {
                session_present: false,
                return_code: ConnectReturnCode::UnacceptableProtocolVersion,
            }));
        }

        // Validate client ID
        let client_id = connect.client_id.clone();
        if client_id.is_empty() && !connect.flags.clean_session {
            // Empty client ID requires clean session
            return HandleResult::Response(Packet::ConnAck(ConnAck {
                session_present: false,
                return_code: ConnectReturnCode::IdentifierRejected,
            }));
        }

        // Initialize the client
        client.initialize(
            client_id,
            connect.flags.clean_session,
            connect.keep_alive,
            connect.username,
        );

        // TODO: Handle session restoration for clean_session=false
        let session_present = false;

        HandleResult::Response(Packet::ConnAck(ConnAck {
            session_present,
            return_code: ConnectReturnCode::Accepted,
        }))
    }

    /// Handle PUBLISH packet.
    fn handle_publish(&self, client: &mut Client, publish: Publish) -> HandleResult {
        if !client.is_connected() {
            return HandleResult::Error("Not connected".to_string());
        }

        client.stats.record_publish();

        // Route the message
        let router = self.router.read();
        let route_result = router.route(
            &publish.topic,
            publish.payload.to_vec(),
            publish.qos,
            publish.retain,
        );

        match route_result {
            Ok(_result) => {
                // Handle QoS acknowledgments
                match publish.qos {
                    QoS::AtMostOnce => HandleResult::None,
                    QoS::AtLeastOnce => {
                        if let Some(packet_id) = publish.packet_id {
                            HandleResult::Response(Packet::PubAck(PubAck { packet_id }))
                        } else {
                            HandleResult::Error("QoS 1 publish missing packet ID".to_string())
                        }
                    }
                    QoS::ExactlyOnce => {
                        if let Some(packet_id) = publish.packet_id {
                            // Store for QoS 2 flow
                            // TODO: Implement full QoS 2 inbound flow
                            HandleResult::Response(Packet::PubRec(PubRec { packet_id }))
                        } else {
                            HandleResult::Error("QoS 2 publish missing packet ID".to_string())
                        }
                    }
                }
            }
            Err(e) => HandleResult::Error(format!("Route error: {}", e)),
        }
    }

    /// Handle PUBACK packet (QoS 1 acknowledgment from client).
    fn handle_puback(&self, client: &mut Client, puback: PubAck) -> HandleResult {
        if !client.is_connected() {
            return HandleResult::Error("Not connected".to_string());
        }

        // Complete QoS 1 outbound flow
        let _ = client.session.on_puback(puback.packet_id);
        HandleResult::None
    }

    /// Handle PUBREC packet (QoS 2, step 2 from client).
    fn handle_pubrec(&self, client: &mut Client, pubrec: PubRec) -> HandleResult {
        if !client.is_connected() {
            return HandleResult::Error("Not connected".to_string());
        }

        // Transition QoS 2 outbound flow
        if client.session.on_pubrec(pubrec.packet_id).is_ok() {
            HandleResult::Response(Packet::PubRel(PubRel { packet_id: pubrec.packet_id }))
        } else {
            // Unknown packet ID - still send PUBREL per spec
            HandleResult::Response(Packet::PubRel(PubRel { packet_id: pubrec.packet_id }))
        }
    }

    /// Handle PUBREL packet (QoS 2, step 3 from client).
    fn handle_pubrel(&self, client: &mut Client, pubrel: PubRel) -> HandleResult {
        if !client.is_connected() {
            return HandleResult::Error("Not connected".to_string());
        }

        // Complete QoS 2 inbound flow
        let _ = client.session.on_pubrel(pubrel.packet_id);
        HandleResult::Response(Packet::PubComp(PubComp { packet_id: pubrel.packet_id }))
    }

    /// Handle PUBCOMP packet (QoS 2, step 4 from client).
    fn handle_pubcomp(&self, client: &mut Client, pubcomp: PubComp) -> HandleResult {
        if !client.is_connected() {
            return HandleResult::Error("Not connected".to_string());
        }

        // Complete QoS 2 outbound flow
        let _ = client.session.on_pubcomp(pubcomp.packet_id);
        HandleResult::None
    }

    /// Handle SUBSCRIBE packet.
    fn handle_subscribe(&self, client: &mut Client, subscribe: Subscribe) -> HandleResult {
        if !client.is_connected() {
            return HandleResult::Error("Not connected".to_string());
        }

        let mut return_codes = Vec::with_capacity(subscribe.subscriptions.len());
        let mut responses = Vec::new();

        let router = self.router.write();

        for sub in &subscribe.subscriptions {
            match router.subscribe(&client.client_id, &sub.topic_filter, sub.qos) {
                Ok((granted_qos, retained_messages)) => {
                    // Add to client's session
                    client.subscribe(&sub.topic_filter, granted_qos);
                    
                    return_codes.push(match granted_qos {
                        QoS::AtMostOnce => SubAckCode::SuccessQoS0,
                        QoS::AtLeastOnce => SubAckCode::SuccessQoS1,
                        QoS::ExactlyOnce => SubAckCode::SuccessQoS2,
                    });

                    // Queue retained messages for delivery
                    for msg in retained_messages {
                        let publish = Publish::new(&msg.topic, msg.payload)
                            .retain(true);
                        responses.push(Packet::Publish(publish));
                    }
                }
                Err(_) => {
                    return_codes.push(SubAckCode::Failure);
                }
            }
        }

        // Build response: SUBACK followed by any retained messages
        let mut all_responses = vec![Packet::SubAck(SubAck {
            packet_id: subscribe.packet_id,
            return_codes,
        })];
        all_responses.extend(responses);

        if all_responses.len() == 1 {
            HandleResult::Response(all_responses.remove(0))
        } else {
            HandleResult::Responses(all_responses)
        }
    }

    /// Handle UNSUBSCRIBE packet.
    fn handle_unsubscribe(&self, client: &mut Client, unsubscribe: Unsubscribe) -> HandleResult {
        if !client.is_connected() {
            return HandleResult::Error("Not connected".to_string());
        }

        let router = self.router.write();

        for filter in &unsubscribe.topic_filters {
            router.unsubscribe(&client.client_id, filter);
            client.unsubscribe(filter);
        }

        HandleResult::Response(Packet::UnsubAck(UnsubAck {
            packet_id: unsubscribe.packet_id,
        }))
    }

    /// Handle PINGREQ packet.
    fn handle_pingreq(&self, client: &mut Client) -> HandleResult {
        if !client.is_connected() {
            return HandleResult::Error("Not connected".to_string());
        }

        HandleResult::Response(Packet::PingResp)
    }

    /// Handle DISCONNECT packet.
    fn handle_disconnect(&self, client: &mut Client) -> HandleResult {
        // Clean disconnect from client
        client.disconnect();

        // Remove subscriptions if clean session
        if client.clean_session {
            let router = self.router.write();
            router.remove_client(&client.client_id);
        }

        HandleResult::Disconnect
    }

    /// Get messages pending for a client.
    pub fn get_pending_messages(&self, client: &mut Client) -> Vec<Packet> {
        // Return any queued messages from the session
        let mut packets = Vec::new();
        
        // Get offline messages if any
        while let Some(msg) = client.session.dequeue_message() {
            let publish = Publish::new(&msg.topic, msg.payload);
            
            // TODO: Assign packet ID for QoS > 0 and track in session
            
            packets.push(Packet::Publish(publish));
        }
        
        packets
    }
}
