//! MQTT packet definitions for all 14 packet types

use bytes::Bytes;

use crate::codec::types::{
    ConnectFlags, ConnectReturnCode, PacketId, QoS, SubAckCode, Subscription, Will,
};

/// All MQTT 3.1.1 packet types
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Packet {
    /// Client request to connect to broker
    Connect(Connect),
    /// Connect acknowledgment
    ConnAck(ConnAck),
    /// Publish message
    Publish(Publish),
    /// Publish acknowledgment (QoS 1)
    PubAck(PubAck),
    /// Publish received (QoS 2, part 1)
    PubRec(PubRec),
    /// Publish release (QoS 2, part 2)
    PubRel(PubRel),
    /// Publish complete (QoS 2, part 3)
    PubComp(PubComp),
    /// Subscribe to topics
    Subscribe(Subscribe),
    /// Subscribe acknowledgment
    SubAck(SubAck),
    /// Unsubscribe from topics
    Unsubscribe(Unsubscribe),
    /// Unsubscribe acknowledgment
    UnsubAck(UnsubAck),
    /// Ping request
    PingReq,
    /// Ping response
    PingResp,
    /// Client disconnecting
    Disconnect,
}

impl Packet {
    /// Get the packet type number (1-14)
    pub fn packet_type(&self) -> u8 {
        match self {
            Packet::Connect(_) => 1,
            Packet::ConnAck(_) => 2,
            Packet::Publish(_) => 3,
            Packet::PubAck(_) => 4,
            Packet::PubRec(_) => 5,
            Packet::PubRel(_) => 6,
            Packet::PubComp(_) => 7,
            Packet::Subscribe(_) => 8,
            Packet::SubAck(_) => 9,
            Packet::Unsubscribe(_) => 10,
            Packet::UnsubAck(_) => 11,
            Packet::PingReq => 12,
            Packet::PingResp => 13,
            Packet::Disconnect => 14,
        }
    }

    /// Get a human-readable name for logging
    pub fn name(&self) -> &'static str {
        match self {
            Packet::Connect(_) => "CONNECT",
            Packet::ConnAck(_) => "CONNACK",
            Packet::Publish(_) => "PUBLISH",
            Packet::PubAck(_) => "PUBACK",
            Packet::PubRec(_) => "PUBREC",
            Packet::PubRel(_) => "PUBREL",
            Packet::PubComp(_) => "PUBCOMP",
            Packet::Subscribe(_) => "SUBSCRIBE",
            Packet::SubAck(_) => "SUBACK",
            Packet::Unsubscribe(_) => "UNSUBSCRIBE",
            Packet::UnsubAck(_) => "UNSUBACK",
            Packet::PingReq => "PINGREQ",
            Packet::PingResp => "PINGRESP",
            Packet::Disconnect => "DISCONNECT",
        }
    }
}

/// CONNECT packet (client → broker)
/// MQTT 3.1.1 §3.1
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Connect {
    /// Protocol name ("MQTT" for 3.1.1)
    pub protocol_name: String,
    /// Protocol level (4 for 3.1.1)
    pub protocol_level: u8,
    /// Connect flags
    pub flags: ConnectFlags,
    /// Keep alive interval in seconds
    pub keep_alive: u16,
    /// Client identifier
    pub client_id: String,
    /// Last will and testament
    pub will: Option<Will>,
    /// Username for authentication
    pub username: Option<String>,
    /// Password for authentication
    pub password: Option<Bytes>,
}

impl Connect {
    /// Create a minimal CONNECT packet
    pub fn new(client_id: impl Into<String>) -> Self {
        Connect {
            protocol_name: "MQTT".to_string(),
            protocol_level: 4,
            flags: ConnectFlags {
                clean_session: true,
                ..Default::default()
            },
            keep_alive: 60,
            client_id: client_id.into(),
            will: None,
            username: None,
            password: None,
        }
    }

    /// Set clean session flag
    pub fn clean_session(mut self, clean: bool) -> Self {
        self.flags.clean_session = clean;
        self
    }

    /// Set keep alive interval
    pub fn keep_alive(mut self, seconds: u16) -> Self {
        self.keep_alive = seconds;
        self
    }

    /// Set will message
    pub fn will(mut self, topic: String, message: Bytes, qos: QoS, retain: bool) -> Self {
        self.flags.will = true;
        self.flags.will_qos = qos;
        self.flags.will_retain = retain;
        self.will = Some(Will {
            topic,
            message,
            qos,
            retain,
        });
        self
    }

    /// Set authentication credentials
    pub fn credentials(mut self, username: String, password: Option<Bytes>) -> Self {
        self.flags.username = true;
        self.flags.password = password.is_some();
        self.username = Some(username);
        self.password = password;
        self
    }
}

/// CONNACK packet (broker → client)
/// MQTT 3.1.1 §3.2
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConnAck {
    /// Session present flag (clean_session=false and session exists)
    pub session_present: bool,
    /// Connect return code
    pub return_code: ConnectReturnCode,
}

impl ConnAck {
    /// Create accepted CONNACK
    pub fn accepted(session_present: bool) -> Self {
        ConnAck {
            session_present,
            return_code: ConnectReturnCode::Accepted,
        }
    }

    /// Create rejected CONNACK
    pub fn rejected(code: ConnectReturnCode) -> Self {
        ConnAck {
            session_present: false,
            return_code: code,
        }
    }
}

/// PUBLISH packet (bidirectional)
/// MQTT 3.1.1 §3.3
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Publish {
    /// Duplicate delivery flag
    pub dup: bool,
    /// Quality of Service level
    pub qos: QoS,
    /// Retain flag
    pub retain: bool,
    /// Topic name
    pub topic: String,
    /// Packet identifier (only for QoS 1/2)
    pub packet_id: Option<PacketId>,
    /// Application message payload
    pub payload: Bytes,
}

impl Publish {
    /// Create a QoS 0 publish
    pub fn new(topic: impl Into<String>, payload: impl Into<Bytes>) -> Self {
        Publish {
            dup: false,
            qos: QoS::AtMostOnce,
            retain: false,
            topic: topic.into(),
            packet_id: None,
            payload: payload.into(),
        }
    }

    /// Create a publish with specific QoS
    pub fn with_qos(
        topic: impl Into<String>,
        payload: impl Into<Bytes>,
        qos: QoS,
        packet_id: PacketId,
    ) -> Self {
        Publish {
            dup: false,
            qos,
            retain: false,
            topic: topic.into(),
            packet_id: if qos == QoS::AtMostOnce {
                None
            } else {
                Some(packet_id)
            },
            payload: payload.into(),
        }
    }

    /// Set retain flag
    pub fn retain(mut self, retain: bool) -> Self {
        self.retain = retain;
        self
    }

    /// Set dup flag (for retransmission)
    pub fn dup(mut self, dup: bool) -> Self {
        self.dup = dup;
        self
    }
}

/// PUBACK packet (QoS 1 acknowledgment)
/// MQTT 3.1.1 §3.4
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PubAck {
    pub packet_id: PacketId,
}

/// PUBREC packet (QoS 2, step 1 response)
/// MQTT 3.1.1 §3.5
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PubRec {
    pub packet_id: PacketId,
}

/// PUBREL packet (QoS 2, step 2)
/// MQTT 3.1.1 §3.6
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PubRel {
    pub packet_id: PacketId,
}

/// PUBCOMP packet (QoS 2, step 3)
/// MQTT 3.1.1 §3.7
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PubComp {
    pub packet_id: PacketId,
}

/// SUBSCRIBE packet (client → broker)
/// MQTT 3.1.1 §3.8
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Subscribe {
    pub packet_id: PacketId,
    pub subscriptions: Vec<Subscription>,
}

impl Subscribe {
    pub fn new(packet_id: PacketId, subscriptions: Vec<Subscription>) -> Self {
        Subscribe {
            packet_id,
            subscriptions,
        }
    }

    /// Create subscribe for a single topic
    pub fn single(packet_id: PacketId, topic: impl Into<String>, qos: QoS) -> Self {
        Subscribe {
            packet_id,
            subscriptions: vec![Subscription {
                topic_filter: topic.into(),
                qos,
            }],
        }
    }
}

/// SUBACK packet (broker → client)
/// MQTT 3.1.1 §3.9
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubAck {
    pub packet_id: PacketId,
    pub return_codes: Vec<SubAckCode>,
}

/// UNSUBSCRIBE packet (client → broker)
/// MQTT 3.1.1 §3.10
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unsubscribe {
    pub packet_id: PacketId,
    pub topic_filters: Vec<String>,
}

impl Unsubscribe {
    pub fn new(packet_id: PacketId, topic_filters: Vec<String>) -> Self {
        Unsubscribe {
            packet_id,
            topic_filters,
        }
    }

    /// Create unsubscribe for a single topic
    pub fn single(packet_id: PacketId, topic: impl Into<String>) -> Self {
        Unsubscribe {
            packet_id,
            topic_filters: vec![topic.into()],
        }
    }
}

/// UNSUBACK packet (broker → client)
/// MQTT 3.1.1 §3.11
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnsubAck {
    pub packet_id: PacketId,
}
