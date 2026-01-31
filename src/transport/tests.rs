//! Tests for the transport module.

use bytes::BytesMut;
use super::*;
use crate::codec::{Packet, Publish, QoS};

mod codec_tests {
    use super::*;

    #[test]
    fn test_decode_empty_buffer() {
        let mut codec = MqttCodec::new();
        let mut buf = BytesMut::new();
        
        let result = codec.decode(&mut buf).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_decode_incomplete_packet() {
        let mut codec = MqttCodec::new();
        let mut buf = BytesMut::from(&[0x10][..]); // CONNECT type, no remaining length
        
        let result = codec.decode(&mut buf).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_encode_and_decode_pingreq() {
        let mut codec = MqttCodec::new();
        let mut buf = BytesMut::new();
        
        // Encode PINGREQ
        let packet = Packet::PingReq;
        codec.encode(&packet, &mut buf).unwrap();
        
        // Should be 2 bytes: 0xC0, 0x00
        assert_eq!(buf.len(), 2);
        assert_eq!(&buf[..], &[0xC0, 0x00]);
        
        // Decode it back
        let decoded = codec.decode(&mut buf).unwrap().unwrap();
        assert!(matches!(decoded, Packet::PingReq));
    }

    #[test]
    fn test_encode_and_decode_pingresp() {
        let mut codec = MqttCodec::new();
        let mut buf = BytesMut::new();
        
        let packet = Packet::PingResp;
        codec.encode(&packet, &mut buf).unwrap();
        
        let decoded = codec.decode(&mut buf).unwrap().unwrap();
        assert!(matches!(decoded, Packet::PingResp));
    }

    #[test]
    fn test_encode_and_decode_disconnect() {
        let mut codec = MqttCodec::new();
        let mut buf = BytesMut::new();
        
        let packet = Packet::Disconnect;
        codec.encode(&packet, &mut buf).unwrap();
        
        let decoded = codec.decode(&mut buf).unwrap().unwrap();
        assert!(matches!(decoded, Packet::Disconnect));
    }

    #[test]
    fn test_encode_and_decode_publish() {
        let mut codec = MqttCodec::new();
        let mut buf = BytesMut::new();
        
        let publish = Publish::with_qos(
            "test/topic",
            b"hello world".as_slice(),
            QoS::AtLeastOnce,
            123,
        );
        let packet = Packet::Publish(publish);
        
        codec.encode(&packet, &mut buf).unwrap();
        
        let decoded = codec.decode(&mut buf).unwrap().unwrap();
        
        match decoded {
            Packet::Publish(p) => {
                assert_eq!(p.topic, "test/topic");
                assert_eq!(p.packet_id, Some(123));
                assert_eq!(&p.payload[..], b"hello world");
            }
            _ => panic!("Expected Publish packet"),
        }
    }

    #[test]
    fn test_decode_multiple_packets() {
        let mut codec = MqttCodec::new();
        let mut buf = BytesMut::new();
        
        // Encode two packets
        codec.encode(&Packet::PingReq, &mut buf).unwrap();
        codec.encode(&Packet::PingResp, &mut buf).unwrap();
        
        assert_eq!(buf.len(), 4); // 2 bytes each
        
        // Decode first
        let p1 = codec.decode(&mut buf).unwrap().unwrap();
        assert!(matches!(p1, Packet::PingReq));
        
        // Decode second
        let p2 = codec.decode(&mut buf).unwrap().unwrap();
        assert!(matches!(p2, Packet::PingResp));
        
        // Buffer should be empty
        assert!(buf.is_empty());
    }

    #[test]
    fn test_packet_size_limit() {
        let mut codec = MqttCodec::with_max_packet_size(100);
        let mut buf = BytesMut::new();
        
        // Create a packet that's too large
        let publish = Publish::new("t", vec![0u8; 200]);
        let packet = Packet::Publish(publish);
        
        let result = codec.encode(&packet, &mut buf);
        assert!(matches!(result, Err(CodecError::PacketTooLarge { .. })));
    }

    #[test]
    fn test_decode_large_remaining_length() {
        let mut codec = MqttCodec::with_max_packet_size(100);
        
        // Construct a packet with remaining length > max (e.g., 1000)
        // Remaining length 1000 = 0xE8 0x07
        let mut buf = BytesMut::from(&[0x30, 0xE8, 0x07][..]);
        
        let result = codec.decode(&mut buf);
        assert!(matches!(result, Err(CodecError::PacketTooLarge { .. })));
    }

    #[test]
    fn test_incremental_decode() {
        let mut codec = MqttCodec::new();
        let mut buf = BytesMut::new();
        
        // Encode a PUBLISH packet
        let publish = Publish::new("test", b"data".as_slice());
        let packet = Packet::Publish(publish);
        
        let mut encoded = BytesMut::new();
        codec.encode(&packet, &mut encoded).unwrap();
        
        // Feed bytes one at a time
        for (i, byte) in encoded.iter().enumerate() {
            buf.extend_from_slice(&[*byte]);
            
            let result = codec.decode(&mut buf).unwrap();
            
            if i < encoded.len() - 1 {
                // Should need more data
                assert!(result.is_none(), "Expected None at byte {}", i);
            } else {
                // Last byte should complete the packet
                assert!(result.is_some(), "Expected Some at last byte");
            }
        }
    }

    #[test]
    fn test_codec_reset_after_error() {
        let mut codec = MqttCodec::with_max_packet_size(10);
        
        // Cause an error with too-large packet
        let mut buf = BytesMut::from(&[0x30, 0x64][..]); // remaining length = 100
        let _ = codec.decode(&mut buf);
        
        // Should be able to decode again
        let mut buf2 = BytesMut::from(&[0xC0, 0x00][..]); // PINGREQ
        let result = codec.decode(&mut buf2).unwrap();
        assert!(matches!(result, Some(Packet::PingReq)));
    }

    #[test]
    fn test_default_max_packet_size() {
        let codec = MqttCodec::new();
        assert_eq!(codec.max_packet_size(), 1024 * 1024);
    }

    #[test]
    fn test_custom_max_packet_size() {
        let codec = MqttCodec::with_max_packet_size(500);
        assert_eq!(codec.max_packet_size(), 500);
    }

    #[test]
    fn test_max_packet_size_capped() {
        // Should be capped at MAX_PACKET_SIZE
        let codec = MqttCodec::with_max_packet_size(usize::MAX);
        assert_eq!(codec.max_packet_size(), MAX_PACKET_SIZE);
    }
}

mod connection_config_tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_default_config() {
        let config = ConnectionConfig::default();
        
        assert_eq!(config.max_packet_size, 1024 * 1024);
        assert_eq!(config.read_timeout, Duration::from_secs(30));
        assert_eq!(config.write_timeout, Duration::from_secs(30));
        assert_eq!(config.read_buffer_size, 8192);
        assert_eq!(config.write_buffer_size, 8192);
    }

    #[test]
    fn test_custom_config() {
        let config = ConnectionConfig {
            max_packet_size: 2048,
            read_timeout: Duration::from_secs(10),
            write_timeout: Duration::from_secs(5),
            read_buffer_size: 4096,
            write_buffer_size: 4096,
        };
        
        assert_eq!(config.max_packet_size, 2048);
        assert_eq!(config.read_timeout, Duration::from_secs(10));
    }
}
