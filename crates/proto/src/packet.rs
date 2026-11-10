// Owned by Pool Agent 1 (arthoroy.in@gmail.com)

use thiserror::Error;

// ----------------------------------------------------------------------------
// 1. Constants
// ----------------------------------------------------------------------------

pub const MAC_LEN: usize = 8;
pub const MAX_PACKET_SIZE: usize = 500;
pub const C2S_HEADER_LEN: usize = 13;
pub const S2C_HEADER_LEN: usize = 11;
pub const MAX_C2S_PAYLOAD: usize = 487;
pub const MAX_S2C_PAYLOAD: usize = 489;
pub const DUMMY_KEY_NONCE: &[u8; 32] = b"c:\\windows\\system\\firewall32.cpl";
pub const TS3_INIT_MAC: &[u8; 8] = b"TS3INIT1";
pub const INIT_VERSION_BYTES: [u8; 4] = [0x06, 0x3b, 0xec, 0xe9];

// ----------------------------------------------------------------------------
// 2. Error Type
// ----------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum PacketError {
    #[error("truncated packet: expected at least {expected} bytes, got {actual}")]
    TruncatedPacket { expected: usize, actual: usize },
    #[error("payload too large: maximum {max} bytes, got {actual}")]
    PayloadTooLarge { max: usize, actual: usize },
    #[error("invalid packet type: {0}")]
    InvalidPacketType(u8),
    #[error("invalid codec: {0}")]
    InvalidCodec(u8),
    #[error("whisper parse error: {0}")]
    WhisperParse(String),
}

// ----------------------------------------------------------------------------
// 3. PacketType
// ----------------------------------------------------------------------------

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PacketType {
    Voice = 0,
    VoiceWhisper = 1,
    Command = 2,
    CommandLow = 3,
    Ping = 4,
    Pong = 5,
    Ack = 6,
    AckLow = 7,
    Init1 = 8,
}

impl PacketType {
    pub fn from_u8(val: u8) -> Result<Self, PacketError> {
        match val {
            0 => Ok(Self::Voice),
            1 => Ok(Self::VoiceWhisper),
            2 => Ok(Self::Command),
            3 => Ok(Self::CommandLow),
            4 => Ok(Self::Ping),
            5 => Ok(Self::Pong),
            6 => Ok(Self::Ack),
            7 => Ok(Self::AckLow),
            8 => Ok(Self::Init1),
            other => Err(PacketError::InvalidPacketType(other)),
        }
    }
}

impl TryFrom<u8> for PacketType {
    type Error = PacketError;

    #[inline]
    fn try_from(val: u8) -> Result<Self, Self::Error> {
        Self::from_u8(val)
    }
}

// ----------------------------------------------------------------------------
// 4. PacketFlags
// ----------------------------------------------------------------------------

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
    pub struct PacketFlags: u8 {
        const NONE         = 0x00;
        const FRAGMENTED   = 0x10;
        const NEW_PROTOCOL = 0x20;
        const COMPRESSED   = 0x40;
        const UNENCRYPTED  = 0x80;
    }
}

// ----------------------------------------------------------------------------
// 5. C2SPacket (Client-to-Server)
// ----------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct C2SPacket {
    pub mac: [u8; 8],
    pub packet_id: u16,
    pub client_id: u16,
    pub packet_type: PacketType,
    pub flags: PacketFlags,
    pub generation_id: u32,
    pub payload: Vec<u8>,
}

impl C2SPacket {
    pub fn new(
        packet_id: u16,
        client_id: u16,
        packet_type: PacketType,
        flags: PacketFlags,
        payload: Vec<u8>,
    ) -> Self {
        Self {
            mac: [0u8; 8],
            packet_id,
            client_id,
            packet_type,
            flags,
            generation_id: 0,
            payload,
        }
    }

    #[inline]
    pub fn type_and_flags(&self) -> u8 {
        (self.flags.bits() & 0xF0) | (self.packet_type as u8 & 0x0F)
    }

    pub fn header_aad(&self) -> [u8; 5] {
        let pid = self.packet_id.to_be_bytes();
        let cid = self.client_id.to_be_bytes();
        [pid[0], pid[1], cid[0], cid[1], self.type_and_flags()]
    }

    pub fn encode(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(C2S_HEADER_LEN + self.payload.len());
        buf.extend_from_slice(&self.mac);
        buf.extend_from_slice(&self.packet_id.to_be_bytes());
        buf.extend_from_slice(&self.client_id.to_be_bytes());
        buf.push(self.type_and_flags());
        buf.extend_from_slice(&self.payload);
        buf
    }

    pub fn decode(raw: &[u8]) -> Result<Self, PacketError> {
        if raw.len() < C2S_HEADER_LEN {
            return Err(PacketError::TruncatedPacket {
                expected: C2S_HEADER_LEN,
                actual: raw.len(),
            });
        }
        if raw.len() > MAX_PACKET_SIZE {
            return Err(PacketError::PayloadTooLarge {
                max: MAX_C2S_PAYLOAD,
                actual: raw.len() - C2S_HEADER_LEN,
            });
        }
        let mut mac = [0u8; 8];
        mac.copy_from_slice(&raw[0..8]);
        let packet_id = u16::from_be_bytes([raw[8], raw[9]]);
        let client_id = u16::from_be_bytes([raw[10], raw[11]]);
        let type_flag_byte = raw[12];
        let packet_type = PacketType::from_u8(type_flag_byte & 0x0F)?;
        let flags = PacketFlags::from_bits_truncate(type_flag_byte & 0xF0);
        let payload = raw[13..].to_vec();
        Ok(Self {
            mac,
            packet_id,
            client_id,
            packet_type,
            flags,
            generation_id: 0,
            payload,
        })
    }
}

// ----------------------------------------------------------------------------
// 6. S2CPacket (Server-to-Client)
// ----------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct S2CPacket {
    pub mac: [u8; 8],
    pub packet_id: u16,
    pub packet_type: PacketType,
    pub flags: PacketFlags,
    pub generation_id: u32,
    pub payload: Vec<u8>,
}

impl S2CPacket {
    pub fn new(
        packet_id: u16,
        packet_type: PacketType,
        flags: PacketFlags,
        payload: Vec<u8>,
    ) -> Self {
        Self {
            mac: [0u8; 8],
            packet_id,
            packet_type,
            flags,
            generation_id: 0,
            payload,
        }
    }

    #[inline]
    pub fn type_and_flags(&self) -> u8 {
        (self.flags.bits() & 0xF0) | (self.packet_type as u8 & 0x0F)
    }

    pub fn header_aad(&self) -> [u8; 3] {
        let pid = self.packet_id.to_be_bytes();
        [pid[0], pid[1], self.type_and_flags()]
    }

    pub fn encode(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(S2C_HEADER_LEN + self.payload.len());
        buf.extend_from_slice(&self.mac);
        buf.extend_from_slice(&self.packet_id.to_be_bytes());
        buf.push(self.type_and_flags());
        buf.extend_from_slice(&self.payload);
        buf
    }

    pub fn decode(raw: &[u8]) -> Result<Self, PacketError> {
        if raw.len() < S2C_HEADER_LEN {
            return Err(PacketError::TruncatedPacket {
                expected: S2C_HEADER_LEN,
                actual: raw.len(),
            });
        }
        if raw.len() > MAX_PACKET_SIZE {
            return Err(PacketError::PayloadTooLarge {
                max: MAX_S2C_PAYLOAD,
                actual: raw.len() - S2C_HEADER_LEN,
            });
        }
        let mut mac = [0u8; 8];
        mac.copy_from_slice(&raw[0..8]);
        let packet_id = u16::from_be_bytes([raw[8], raw[9]]);
        let type_flag_byte = raw[10];
        let packet_type = PacketType::from_u8(type_flag_byte & 0x0F)?;
        let flags = PacketFlags::from_bits_truncate(type_flag_byte & 0xF0);
        let payload = raw[11..].to_vec();
        Ok(Self {
            mac,
            packet_id,
            packet_type,
            flags,
            generation_id: 0,
            payload,
        })
    }
}

// ----------------------------------------------------------------------------
// 7. Voice Types
// ----------------------------------------------------------------------------

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CodecType {
    SpeexNarrowband = 0,
    SpeexWideband = 1,
    SpeexUltraWideband = 2,
    CeltMono = 3,
    OpusVoice = 4,
    OpusMusic = 5,
}

impl CodecType {
    pub fn from_u8(val: u8) -> Result<Self, PacketError> {
        match val {
            0 => Ok(Self::SpeexNarrowband),
            1 => Ok(Self::SpeexWideband),
            2 => Ok(Self::SpeexUltraWideband),
            3 => Ok(Self::CeltMono),
            4 => Ok(Self::OpusVoice),
            5 => Ok(Self::OpusMusic),
            other => Err(PacketError::InvalidCodec(other)),
        }
    }
}

impl TryFrom<u8> for CodecType {
    type Error = PacketError;

    #[inline]
    fn try_from(val: u8) -> Result<Self, Self::Error> {
        Self::from_u8(val)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VoicePacketC2S {
    pub voice_id: u16,
    pub codec: CodecType,
    pub audio_data: Vec<u8>,
}

impl VoicePacketC2S {
    pub fn new(voice_id: u16, codec: CodecType, audio_data: Vec<u8>) -> Self {
        Self {
            voice_id,
            codec,
            audio_data,
        }
    }

    pub fn stop_talking(voice_id: u16, codec: CodecType) -> Self {
        Self {
            voice_id,
            codec,
            audio_data: Vec::new(),
        }
    }

    #[inline]
    pub fn is_stop_talking(&self) -> bool {
        self.audio_data.is_empty()
    }

    pub fn encode(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(3 + self.audio_data.len());
        buf.extend_from_slice(&self.voice_id.to_be_bytes());
        buf.push(self.codec as u8);
        buf.extend_from_slice(&self.audio_data);
        buf
    }

    pub fn decode(raw: &[u8]) -> Result<Self, PacketError> {
        if raw.len() < 3 {
            return Err(PacketError::TruncatedPacket {
                expected: 3,
                actual: raw.len(),
            });
        }
        if raw.len() > MAX_C2S_PAYLOAD {
            return Err(PacketError::PayloadTooLarge {
                max: MAX_C2S_PAYLOAD,
                actual: raw.len(),
            });
        }
        let voice_id = u16::from_be_bytes([raw[0], raw[1]]);
        let codec = CodecType::from_u8(raw[2])?;
        let audio_data = raw[3..].to_vec();
        Ok(Self {
            voice_id,
            codec,
            audio_data,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VoicePacketS2C {
    pub voice_id: u16,
    pub talker_client_id: u16,
    pub codec: CodecType,
    pub audio_data: Vec<u8>,
}

impl VoicePacketS2C {
    pub fn new(
        voice_id: u16,
        talker_client_id: u16,
        codec: CodecType,
        audio_data: Vec<u8>,
    ) -> Self {
        Self {
            voice_id,
            talker_client_id,
            codec,
            audio_data,
        }
    }

    #[inline]
    pub fn is_stop_talking(&self) -> bool {
        self.audio_data.is_empty()
    }

    pub fn encode(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(5 + self.audio_data.len());
        buf.extend_from_slice(&self.voice_id.to_be_bytes());
        buf.extend_from_slice(&self.talker_client_id.to_be_bytes());
        buf.push(self.codec as u8);
        buf.extend_from_slice(&self.audio_data);
        buf
    }

    pub fn decode(raw: &[u8]) -> Result<Self, PacketError> {
        if raw.len() < 5 {
            return Err(PacketError::TruncatedPacket {
                expected: 5,
                actual: raw.len(),
            });
        }
        if raw.len() > MAX_S2C_PAYLOAD {
            return Err(PacketError::PayloadTooLarge {
                max: MAX_S2C_PAYLOAD,
                actual: raw.len(),
            });
        }
        let voice_id = u16::from_be_bytes([raw[0], raw[1]]);
        let talker_client_id = u16::from_be_bytes([raw[2], raw[3]]);
        let codec = CodecType::from_u8(raw[4])?;
        let audio_data = raw[5..].to_vec();
        Ok(Self {
            voice_id,
            talker_client_id,
            codec,
            audio_data,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WhisperPacketC2S {
    pub voice_id: u16,
    pub codec: CodecType,
    pub channel_ids: Vec<u64>,
    pub client_ids: Vec<u16>,
    pub audio_data: Vec<u8>,
}

impl WhisperPacketC2S {
    pub fn new(
        voice_id: u16,
        codec: CodecType,
        channel_ids: Vec<u64>,
        client_ids: Vec<u16>,
        audio_data: Vec<u8>,
    ) -> Self {
        Self {
            voice_id,
            codec,
            channel_ids,
            client_ids,
            audio_data,
        }
    }

    #[inline]
    pub fn is_stop_talking(&self) -> bool {
        self.audio_data.is_empty()
    }

    pub fn encode(&self) -> Vec<u8> {
        let num_chan = self.channel_ids.len().min(255);
        let num_cli = self.client_ids.len().min(255);
        let mut buf = Vec::with_capacity(5 + num_chan * 8 + num_cli * 2 + self.audio_data.len());
        buf.extend_from_slice(&self.voice_id.to_be_bytes());
        buf.push(self.codec as u8);
        buf.push(num_chan as u8);
        buf.push(num_cli as u8);
        for &cid in &self.channel_ids[..num_chan] {
            buf.extend_from_slice(&cid.to_be_bytes());
        }
        for &cli in &self.client_ids[..num_cli] {
            buf.extend_from_slice(&cli.to_be_bytes());
        }
        buf.extend_from_slice(&self.audio_data);
        buf
    }

    pub fn decode(raw: &[u8]) -> Result<Self, PacketError> {
        if raw.len() < 5 {
            return Err(PacketError::TruncatedPacket {
                expected: 5,
                actual: raw.len(),
            });
        }
        if raw.len() > MAX_C2S_PAYLOAD {
            return Err(PacketError::PayloadTooLarge {
                max: MAX_C2S_PAYLOAD,
                actual: raw.len(),
            });
        }
        let voice_id = u16::from_be_bytes([raw[0], raw[1]]);
        let codec = CodecType::from_u8(raw[2])?;
        let channel_count = raw[3] as usize;
        let client_count = raw[4] as usize;
        let expected_header_len = 5 + channel_count * 8 + client_count * 2;
        if raw.len() < expected_header_len {
            return Err(PacketError::WhisperParse(format!(
                "buffer truncated: expected at least {expected_header_len} bytes for whisper targets, got {}",
                raw.len()
            )));
        }
        let mut offset = 5;
        let mut channel_ids = Vec::with_capacity(channel_count);
        for _ in 0..channel_count {
            let mut cid_bytes = [0u8; 8];
            cid_bytes.copy_from_slice(&raw[offset..offset + 8]);
            channel_ids.push(u64::from_be_bytes(cid_bytes));
            offset += 8;
        }
        let mut client_ids = Vec::with_capacity(client_count);
        for _ in 0..client_count {
            let mut clid_bytes = [0u8; 2];
            clid_bytes.copy_from_slice(&raw[offset..offset + 2]);
            client_ids.push(u16::from_be_bytes(clid_bytes));
            offset += 2;
        }
        let audio_data = raw[offset..].to_vec();
        Ok(Self {
            voice_id,
            codec,
            channel_ids,
            client_ids,
            audio_data,
        })
    }
}

// ----------------------------------------------------------------------------
// 8. Unit Tests
// ----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constants() {
        assert_eq!(MAC_LEN, 8);
        assert_eq!(MAX_PACKET_SIZE, 500);
        assert_eq!(C2S_HEADER_LEN, 13);
        assert_eq!(S2C_HEADER_LEN, 11);
        assert_eq!(MAX_C2S_PAYLOAD, 487);
        assert_eq!(MAX_S2C_PAYLOAD, 489);
        assert_eq!(DUMMY_KEY_NONCE, b"c:\\windows\\system\\firewall32.cpl");
        assert_eq!(TS3_INIT_MAC, b"TS3INIT1");
        assert_eq!(INIT_VERSION_BYTES, [0x06, 0x3b, 0xec, 0xe9]);
    }

    #[test]
    fn test_packet_type_from_u8() {
        assert_eq!(PacketType::from_u8(0).unwrap(), PacketType::Voice);
        assert_eq!(PacketType::from_u8(1).unwrap(), PacketType::VoiceWhisper);
        assert_eq!(PacketType::from_u8(2).unwrap(), PacketType::Command);
        assert_eq!(PacketType::from_u8(3).unwrap(), PacketType::CommandLow);
        assert_eq!(PacketType::from_u8(4).unwrap(), PacketType::Ping);
        assert_eq!(PacketType::from_u8(5).unwrap(), PacketType::Pong);
        assert_eq!(PacketType::from_u8(6).unwrap(), PacketType::Ack);
        assert_eq!(PacketType::from_u8(7).unwrap(), PacketType::AckLow);
        assert_eq!(PacketType::from_u8(8).unwrap(), PacketType::Init1);

        for invalid in 9..=255 {
            assert_eq!(
                PacketType::from_u8(invalid).unwrap_err(),
                PacketError::InvalidPacketType(invalid)
            );
        }
    }

    #[test]
    fn test_codec_type_from_u8() {
        assert_eq!(CodecType::from_u8(0).unwrap(), CodecType::SpeexNarrowband);
        assert_eq!(CodecType::from_u8(1).unwrap(), CodecType::SpeexWideband);
        assert_eq!(CodecType::from_u8(2).unwrap(), CodecType::SpeexUltraWideband);
        assert_eq!(CodecType::from_u8(3).unwrap(), CodecType::CeltMono);
        assert_eq!(CodecType::from_u8(4).unwrap(), CodecType::OpusVoice);
        assert_eq!(CodecType::from_u8(5).unwrap(), CodecType::OpusMusic);

        for invalid in 6..=255 {
            assert_eq!(
                CodecType::from_u8(invalid).unwrap_err(),
                PacketError::InvalidCodec(invalid)
            );
        }
    }

    #[test]
    fn test_packet_flags_combinations() {
        assert_eq!(PacketFlags::NONE.bits(), 0x00);
        assert_eq!(PacketFlags::FRAGMENTED.bits(), 0x10);
        assert_eq!(PacketFlags::NEW_PROTOCOL.bits(), 0x20);
        assert_eq!(PacketFlags::COMPRESSED.bits(), 0x40);
        assert_eq!(PacketFlags::UNENCRYPTED.bits(), 0x80);

        let combo = PacketFlags::NEW_PROTOCOL | PacketFlags::COMPRESSED;
        assert_eq!(combo.bits(), 0x60);
        assert!(combo.contains(PacketFlags::NEW_PROTOCOL));
        assert!(combo.contains(PacketFlags::COMPRESSED));
        assert!(!combo.contains(PacketFlags::UNENCRYPTED));

        let all = PacketFlags::FRAGMENTED
            | PacketFlags::NEW_PROTOCOL
            | PacketFlags::COMPRESSED
            | PacketFlags::UNENCRYPTED;
        assert_eq!(all.bits(), 0xF0);
    }

    #[test]
    fn test_c2s_header_encode_decode_roundtrip() {
        let mut pkt = C2SPacket::new(
            0x1234,
            0x5678,
            PacketType::Command,
            PacketFlags::NEW_PROTOCOL | PacketFlags::COMPRESSED,
            vec![10, 20, 30, 40, 50],
        );
        pkt.mac = [1, 2, 3, 4, 5, 6, 7, 8];

        let encoded = pkt.encode();
        assert_eq!(encoded.len(), C2S_HEADER_LEN + 5);
        assert_eq!(&encoded[0..8], &[1, 2, 3, 4, 5, 6, 7, 8]);
        assert_eq!(&encoded[8..10], &0x1234u16.to_be_bytes());
        assert_eq!(&encoded[10..12], &0x5678u16.to_be_bytes());
        assert_eq!(encoded[12], 0x62); // 0x60 (flags) | 0x02 (Command)
        assert_eq!(&encoded[13..], &[10, 20, 30, 40, 50]);

        let decoded = C2SPacket::decode(&encoded).expect("failed to decode valid C2SPacket");
        assert_eq!(decoded.mac, pkt.mac);
        assert_eq!(decoded.packet_id, pkt.packet_id);
        assert_eq!(decoded.client_id, pkt.client_id);
        assert_eq!(decoded.packet_type, pkt.packet_type);
        assert_eq!(decoded.flags, pkt.flags);
        assert_eq!(decoded.payload, pkt.payload);
    }

    #[test]
    fn test_c2s_empty_payload_roundtrip() {
        let pkt = C2SPacket::new(
            100,
            200,
            PacketType::Ping,
            PacketFlags::UNENCRYPTED,
            Vec::new(),
        );
        let encoded = pkt.encode();
        assert_eq!(encoded.len(), C2S_HEADER_LEN);
        let decoded = C2SPacket::decode(&encoded).unwrap();
        assert_eq!(decoded.payload, Vec::<u8>::new());
        assert_eq!(decoded.packet_type, PacketType::Ping);
        assert_eq!(decoded.flags, PacketFlags::UNENCRYPTED);
    }

    #[test]
    fn test_c2s_header_aad() {
        let pkt = C2SPacket::new(
            0x0102,
            0x0304,
            PacketType::Ack,
            PacketFlags::NEW_PROTOCOL,
            vec![1, 2],
        );
        let aad = pkt.header_aad();
        assert_eq!(aad.len(), 5);
        assert_eq!(aad, [0x01, 0x02, 0x03, 0x04, 0x26]);

        let encoded = pkt.encode();
        assert_eq!(&encoded[8..13], &aad);
    }

    #[test]
    fn test_c2s_decode_error_cases() {
        // Truncated packet (< 13 bytes)
        let short = vec![0u8; 12];
        assert_eq!(
            C2SPacket::decode(&short).unwrap_err(),
            PacketError::TruncatedPacket {
                expected: C2S_HEADER_LEN,
                actual: 12,
            }
        );

        // Payload too large (> 500 bytes total, payload > 487)
        let too_large = vec![0u8; 501];
        assert_eq!(
            C2SPacket::decode(&too_large).unwrap_err(),
            PacketError::PayloadTooLarge {
                max: MAX_C2S_PAYLOAD,
                actual: 501 - C2S_HEADER_LEN,
            }
        );

        // Invalid packet type in header
        let mut invalid_type = vec![0u8; 13];
        invalid_type[12] = 0x09; // type 9 is invalid
        assert_eq!(
            C2SPacket::decode(&invalid_type).unwrap_err(),
            PacketError::InvalidPacketType(9)
        );
    }

    #[test]
    fn test_s2c_header_encode_decode_roundtrip() {
        let mut pkt = S2CPacket::new(
            0xABCD,
            PacketType::Pong,
            PacketFlags::UNENCRYPTED,
            vec![0x12, 0x34],
        );
        pkt.mac = [8, 7, 6, 5, 4, 3, 2, 1];

        let encoded = pkt.encode();
        assert_eq!(encoded.len(), S2C_HEADER_LEN + 2);
        assert_eq!(&encoded[0..8], &[8, 7, 6, 5, 4, 3, 2, 1]);
        assert_eq!(&encoded[8..10], &0xABCDu16.to_be_bytes());
        assert_eq!(encoded[10], 0x85); // 0x80 (unencrypted) | 0x05 (Pong)
        assert_eq!(&encoded[11..], &[0x12, 0x34]);

        let decoded = S2CPacket::decode(&encoded).expect("failed to decode valid S2CPacket");
        assert_eq!(decoded.mac, pkt.mac);
        assert_eq!(decoded.packet_id, pkt.packet_id);
        assert_eq!(decoded.packet_type, pkt.packet_type);
        assert_eq!(decoded.flags, pkt.flags);
        assert_eq!(decoded.payload, pkt.payload);
    }

    #[test]
    fn test_s2c_empty_payload_roundtrip() {
        let pkt = S2CPacket::new(5, PacketType::Ping, PacketFlags::NONE, Vec::new());
        let encoded = pkt.encode();
        assert_eq!(encoded.len(), S2C_HEADER_LEN);
        let decoded = S2CPacket::decode(&encoded).unwrap();
        assert_eq!(decoded.payload, Vec::<u8>::new());
        assert_eq!(decoded.packet_type, PacketType::Ping);
    }

    #[test]
    fn test_s2c_header_aad() {
        let pkt = S2CPacket::new(
            0x1122,
            PacketType::Init1,
            PacketFlags::UNENCRYPTED,
            vec![],
        );
        let aad = pkt.header_aad();
        assert_eq!(aad.len(), 3);
        assert_eq!(aad, [0x11, 0x22, 0x88]);

        let encoded = pkt.encode();
        assert_eq!(&encoded[8..11], &aad);
    }

    #[test]
    fn test_s2c_decode_error_cases() {
        // Truncated packet (< 11 bytes)
        let short = vec![0u8; 10];
        assert_eq!(
            S2CPacket::decode(&short).unwrap_err(),
            PacketError::TruncatedPacket {
                expected: S2C_HEADER_LEN,
                actual: 10,
            }
        );

        // Payload too large (> 500 bytes total, payload > 489)
        let too_large = vec![0u8; 501];
        assert_eq!(
            S2CPacket::decode(&too_large).unwrap_err(),
            PacketError::PayloadTooLarge {
                max: MAX_S2C_PAYLOAD,
                actual: 501 - S2C_HEADER_LEN,
            }
        );

        // Invalid packet type in header
        let mut invalid_type = vec![0u8; 11];
        invalid_type[10] = 0x0E;
        assert_eq!(
            S2CPacket::decode(&invalid_type).unwrap_err(),
            PacketError::InvalidPacketType(14)
        );
    }

    #[test]
    fn test_voice_c2s_encode_decode_roundtrip_and_stop_talking() {
        let voice = VoicePacketC2S::new(42, CodecType::OpusVoice, vec![0xDE, 0xAD, 0xBE, 0xEF]);
        assert!(!voice.is_stop_talking());

        let encoded = voice.encode();
        assert_eq!(encoded.len(), 3 + 4);
        assert_eq!(&encoded[0..2], &42u16.to_be_bytes());
        assert_eq!(encoded[2], CodecType::OpusVoice as u8);
        assert_eq!(&encoded[3..], &[0xDE, 0xAD, 0xBE, 0xEF]);

        let decoded = VoicePacketC2S::decode(&encoded).unwrap();
        assert_eq!(decoded.voice_id, 42);
        assert_eq!(decoded.codec, CodecType::OpusVoice);
        assert_eq!(decoded.audio_data, vec![0xDE, 0xAD, 0xBE, 0xEF]);
        assert!(!decoded.is_stop_talking());

        // Stop talking
        let stop = VoicePacketC2S::stop_talking(43, CodecType::OpusVoice);
        assert!(stop.is_stop_talking());
        let stop_encoded = stop.encode();
        assert_eq!(stop_encoded.len(), 3);
        let stop_decoded = VoicePacketC2S::decode(&stop_encoded).unwrap();
        assert!(stop_decoded.is_stop_talking());
        assert_eq!(stop_decoded.voice_id, 43);
    }

    #[test]
    fn test_voice_c2s_decode_errors() {
        // Truncated (< 3 bytes)
        assert_eq!(
            VoicePacketC2S::decode(&[0x00, 0x01]).unwrap_err(),
            PacketError::TruncatedPacket {
                expected: 3,
                actual: 2,
            }
        );

        // Invalid codec
        assert_eq!(
            VoicePacketC2S::decode(&[0x00, 0x01, 0xFF]).unwrap_err(),
            PacketError::InvalidCodec(0xFF)
        );

        // Payload too large (> 487 bytes)
        let too_large = vec![0u8; 488];
        assert_eq!(
            VoicePacketC2S::decode(&too_large).unwrap_err(),
            PacketError::PayloadTooLarge {
                max: MAX_C2S_PAYLOAD,
                actual: 488,
            }
        );
    }

    #[test]
    fn test_voice_s2c_encode_decode_roundtrip_and_stop_talking() {
        let voice = VoicePacketS2C::new(
            101,
            202,
            CodecType::OpusMusic,
            vec![1, 2, 3, 4, 5],
        );
        assert!(!voice.is_stop_talking());

        let encoded = voice.encode();
        assert_eq!(encoded.len(), 5 + 5);
        assert_eq!(&encoded[0..2], &101u16.to_be_bytes());
        assert_eq!(&encoded[2..4], &202u16.to_be_bytes());
        assert_eq!(encoded[4], CodecType::OpusMusic as u8);
        assert_eq!(&encoded[5..], &[1, 2, 3, 4, 5]);

        let decoded = VoicePacketS2C::decode(&encoded).unwrap();
        assert_eq!(decoded.voice_id, 101);
        assert_eq!(decoded.talker_client_id, 202);
        assert_eq!(decoded.codec, CodecType::OpusMusic);
        assert_eq!(decoded.audio_data, vec![1, 2, 3, 4, 5]);
        assert!(!decoded.is_stop_talking());

        // Stop talking (empty audio)
        let stop = VoicePacketS2C::new(102, 202, CodecType::OpusMusic, Vec::new());
        assert!(stop.is_stop_talking());
        let stop_encoded = stop.encode();
        assert_eq!(stop_encoded.len(), 5);
        let stop_decoded = VoicePacketS2C::decode(&stop_encoded).unwrap();
        assert!(stop_decoded.is_stop_talking());
    }

    #[test]
    fn test_voice_s2c_decode_errors() {
        // Truncated (< 5 bytes)
        assert_eq!(
            VoicePacketS2C::decode(&[0, 1, 0, 2]).unwrap_err(),
            PacketError::TruncatedPacket {
                expected: 5,
                actual: 4,
            }
        );

        // Invalid codec
        assert_eq!(
            VoicePacketS2C::decode(&[0, 1, 0, 2, 99]).unwrap_err(),
            PacketError::InvalidCodec(99)
        );

        // Payload too large (> 489 bytes)
        let too_large = vec![0u8; 490];
        assert_eq!(
            VoicePacketS2C::decode(&too_large).unwrap_err(),
            PacketError::PayloadTooLarge {
                max: MAX_S2C_PAYLOAD,
                actual: 490,
            }
        );
    }

    #[test]
    fn test_whisper_c2s_encode_decode_roundtrip() {
        let channel_ids = vec![10001u64, 10002u64];
        let client_ids = vec![501u16, 502u16, 503u16];
        let audio_data = vec![0x11, 0x22, 0x33, 0x44];

        let whisper = WhisperPacketC2S::new(
            999,
            CodecType::OpusVoice,
            channel_ids.clone(),
            client_ids.clone(),
            audio_data.clone(),
        );

        let encoded = whisper.encode();
        // 5 (prefix) + 2*8 (channels) + 3*2 (clients) + 4 (audio) = 5 + 16 + 6 + 4 = 31 bytes
        assert_eq!(encoded.len(), 31);
        assert_eq!(&encoded[0..2], &999u16.to_be_bytes());
        assert_eq!(encoded[2], CodecType::OpusVoice as u8);
        assert_eq!(encoded[3], 2); // num channels
        assert_eq!(encoded[4], 3); // num clients

        let decoded = WhisperPacketC2S::decode(&encoded).unwrap();
        assert_eq!(decoded.voice_id, 999);
        assert_eq!(decoded.codec, CodecType::OpusVoice);
        assert_eq!(decoded.channel_ids, channel_ids);
        assert_eq!(decoded.client_ids, client_ids);
        assert_eq!(decoded.audio_data, audio_data);

        // Empty targets and empty audio
        let empty_whisper = WhisperPacketC2S::new(
            1,
            CodecType::SpeexWideband,
            Vec::new(),
            Vec::new(),
            Vec::new(),
        );
        let empty_encoded = empty_whisper.encode();
        assert_eq!(empty_encoded.len(), 5);
        let empty_decoded = WhisperPacketC2S::decode(&empty_encoded).unwrap();
        assert_eq!(empty_decoded.channel_ids.len(), 0);
        assert_eq!(empty_decoded.client_ids.len(), 0);
        assert_eq!(empty_decoded.audio_data.len(), 0);
    }

    #[test]
    fn test_whisper_c2s_decode_errors() {
        // Truncated (< 5 bytes)
        assert_eq!(
            WhisperPacketC2S::decode(&[0, 1, 4, 1]).unwrap_err(),
            PacketError::TruncatedPacket {
                expected: 5,
                actual: 4,
            }
        );

        // Invalid codec
        assert_eq!(
            WhisperPacketC2S::decode(&[0, 1, 7, 0, 0]).unwrap_err(),
            PacketError::InvalidCodec(7)
        );

        // Incomplete targets (claims 2 channels = 16B, but only 5B provided)
        let truncated_targets = [0x00, 0x01, 0x04, 0x02, 0x00];
        assert!(matches!(
            WhisperPacketC2S::decode(&truncated_targets).unwrap_err(),
            PacketError::WhisperParse(_)
        ));

        // Payload too large (> 487 bytes)
        let too_large = vec![0u8; 488];
        assert_eq!(
            WhisperPacketC2S::decode(&too_large).unwrap_err(),
            PacketError::PayloadTooLarge {
                max: MAX_C2S_PAYLOAD,
                actual: 488,
            }
        );
    }
}
