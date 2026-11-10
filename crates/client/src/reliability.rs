// Owned by Pool Agent 6 (arthoroy28a@gmail.com)

use std::time::{Duration, Instant};
use ts3_proto::packet::{PacketFlags, PacketType, MAX_C2S_PAYLOAD};
use ts3_proto::quicklz::{QuickLzError, MAX_DECOMPRESSED_SIZE};

// ----------------------------------------------------------------------------
// 1. ReliabilityError
// ----------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ReliabilityError {
    #[error("packet {packet_type:?} (id {packet_id}) timed out after {elapsed:?}")]
    PacketTimeout {
        packet_type: PacketType,
        packet_id: u16,
        elapsed: Duration,
    },
    #[error("decompression error: {0}")]
    Decompression(#[from] QuickLzError),
    #[error("invalid utf-8: {0}")]
    InvalidUtf8(#[from] std::string::FromUtf8Error),
}

// ----------------------------------------------------------------------------
// 2. Constants
// ----------------------------------------------------------------------------

pub const MOD_VAL: u32 = 65536;
pub const VOICE_WINDOW_SIZE: u16 = 32768;
pub const COMMAND_WINDOW_SIZE: u16 = 128;
pub const CLOCK_RESOLUTION: Duration = Duration::from_millis(100);
pub const MAX_RETRY_INTERVAL: Duration = Duration::from_millis(1000);
pub const PACKET_TIMEOUT: Duration = Duration::from_secs(30);
pub const MAX_C2S_CHUNK_SIZE: usize = MAX_C2S_PAYLOAD;
pub const PING_INTERVAL: Duration = Duration::from_secs(1);
pub const ALPHA: f64 = 0.125;
pub const BETA: f64 = 0.25;

// ----------------------------------------------------------------------------
// 3. GenerationWindow
// ----------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GenerationWindow {
    pub mapped_base_offset: u32,
    pub generation: u32,
    pub window_size: u32,
}

impl GenerationWindow {
    pub fn new(window_size: u16) -> Self {
        Self {
            mapped_base_offset: 0,
            generation: 0,
            window_size: window_size as u32,
        }
    }

    pub fn voice() -> Self {
        Self::new(VOICE_WINDOW_SIZE)
    }

    pub fn command() -> Self {
        Self::new(COMMAND_WINDOW_SIZE)
    }

    pub fn new_voice() -> Self {
        Self::voice()
    }

    pub fn new_command() -> Self {
        Self::command()
    }

    pub fn is_in_window(&self, packet_id: u16) -> bool {
        let p = packet_id as u32;
        let max_offset = self.mapped_base_offset + self.window_size;
        if max_offset < MOD_VAL {
            p >= self.mapped_base_offset && p < max_offset
        } else {
            p >= self.mapped_base_offset || p < (max_offset - MOD_VAL)
        }
    }

    pub fn is_next_gen(&self, packet_id: u16) -> bool {
        let p = packet_id as u32;
        self.mapped_base_offset > (MOD_VAL - self.window_size)
            && p < (self.mapped_base_offset + self.window_size) - MOD_VAL
    }

    pub fn get_generation(&self, packet_id: u16) -> Option<u32> {
        if self.is_in_window(packet_id) {
            Some(self.generation_unchecked(packet_id))
        } else {
            None
        }
    }

    pub fn generation_unchecked(&self, packet_id: u16) -> u32 {
        self.generation.wrapping_add(if self.is_next_gen(packet_id) { 1 } else { 0 })
    }

    pub fn resolve_generation(&self, packet_id: u16) -> u32 {
        self.generation_unchecked(packet_id)
    }

    pub fn mapped_to_index(&self, packet_id: u16) -> isize {
        let p = packet_id as isize;
        let base = self.mapped_base_offset as isize;
        if self.is_next_gen(packet_id) {
            (p + MOD_VAL as isize) - base
        } else {
            p - base
        }
    }

    pub fn advance(&mut self, amount: u32) {
        let new_base = self.mapped_base_offset + amount;
        self.generation = self.generation.wrapping_add(new_base / MOD_VAL);
        self.mapped_base_offset = new_base % MOD_VAL;
    }

    pub fn advance_to_excluded(&mut self, packet_id: u16) {
        let idx = self.mapped_to_index(packet_id);
        if idx >= 0 {
            self.advance((idx as u32) + 1);
        }
    }

    pub fn accept_and_advance(&mut self, packet_id: u16) -> Option<u32> {
        if let Some(gen) = self.get_generation(packet_id) {
            self.advance_to_excluded(packet_id);
            Some(gen)
        } else {
            None
        }
    }

    pub fn set_and_drag(&mut self, packet_id: u16) -> bool {
        self.accept_and_advance(packet_id).is_some()
    }

    pub fn reset(&mut self) {
        self.mapped_base_offset = 0;
        self.generation = 0;
    }
}

impl Default for GenerationWindow {
    fn default() -> Self {
        Self::command()
    }
}

// ----------------------------------------------------------------------------
// 4. OutSequenceTracker
// ----------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutSequenceTracker {
    pub packet_counter: [u16; 9],
    pub generation_counter: [u32; 9],
}

impl OutSequenceTracker {
    pub fn new() -> Self {
        Self {
            packet_counter: [0; 9],
            generation_counter: [0; 9],
        }
    }

    pub fn next(&mut self, packet_type: PacketType) -> (u16, u32) {
        if packet_type == PacketType::Init1 {
            return (101, 0);
        }
        let idx = packet_type as usize;
        let id = self.packet_counter[idx];
        let gen = self.generation_counter[idx];
        self.packet_counter[idx] = id.wrapping_add(1);
        if self.packet_counter[idx] == 0 {
            self.generation_counter[idx] = gen.wrapping_add(1);
        }
        (id, gen)
    }

    pub fn next_id(&mut self, packet_type: PacketType) -> (u16, u32) {
        self.next(packet_type)
    }

    pub fn peek(&self, packet_type: PacketType) -> (u16, u32) {
        if packet_type == PacketType::Init1 {
            (101, 0)
        } else {
            let idx = packet_type as usize;
            (self.packet_counter[idx], self.generation_counter[idx])
        }
    }

    pub fn skip_command_id_zero(&mut self) {
        let idx = PacketType::Command as usize;
        if self.packet_counter[idx] == 0 {
            self.packet_counter[idx] = 1;
        }
    }

    pub fn reset(&mut self) {
        self.packet_counter = [0; 9];
        self.generation_counter = [0; 9];
    }
}

impl Default for OutSequenceTracker {
    fn default() -> Self {
        Self::new()
    }
}

// ----------------------------------------------------------------------------
// 5. RttEstimator
// ----------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub struct RttEstimator {
    pub srtt: Option<Duration>,
    pub rttvar: Duration,
    pub rto: Duration,
}

impl RttEstimator {
    pub fn new() -> Self {
        Self {
            srtt: None,
            rttvar: Duration::ZERO,
            rto: MAX_RETRY_INTERVAL,
        }
    }

    pub fn update(&mut self, sample_rtt: Duration) {
        if let Some(srtt) = self.srtt {
            let srtt_secs = (1.0 - ALPHA) * srtt.as_secs_f64() + ALPHA * sample_rtt.as_secs_f64();
            let new_srtt = Duration::from_secs_f64(srtt_secs);
            self.srtt = Some(new_srtt);
            let diff = (sample_rtt.as_secs_f64() - new_srtt.as_secs_f64()).abs();
            let rttvar_secs = (1.0 - BETA) * self.rttvar.as_secs_f64() + BETA * diff;
            self.rttvar = Duration::from_secs_f64(rttvar_secs);
        } else {
            self.srtt = Some(sample_rtt);
            self.rttvar = Duration::ZERO;
        }
        let var_term = (4.0 * self.rttvar.as_secs_f64()).max(CLOCK_RESOLUTION.as_secs_f64());
        let base = self.srtt.unwrap_or(MAX_RETRY_INTERVAL);
        self.rto = base + Duration::from_secs_f64(var_term);
    }

    pub fn record_sample(&mut self, sample_rtt: Duration) {
        self.update(sample_rtt);
    }

    pub fn backoff(&mut self) -> Duration {
        self.rto = (self.rto.saturating_mul(2)).min(MAX_RETRY_INTERVAL);
        self.rto
    }

    pub fn on_resend(&mut self) -> Duration {
        self.backoff()
    }

    pub fn rto(&self) -> Duration {
        self.rto
    }

    pub fn srtt(&self) -> Duration {
        self.srtt.unwrap_or(MAX_RETRY_INTERVAL)
    }

    pub fn srtt_or_default(&self) -> Duration {
        self.srtt()
    }

    pub fn rttvar(&self) -> Duration {
        self.rttvar
    }

    pub fn reset(&mut self) {
        self.srtt = None;
        self.rttvar = Duration::ZERO;
        self.rto = MAX_RETRY_INTERVAL;
    }
}

impl Default for RttEstimator {
    fn default() -> Self {
        Self::new()
    }
}

// ----------------------------------------------------------------------------
// 6. OutQueue and OutPacketEntry
// ----------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct OutPacketEntry {
    pub packet_id: u16,
    pub packet_type: PacketType,
    pub encoded_bytes: Vec<u8>,
    pub first_sent: Instant,
    pub last_sent: Instant,
    pub rto: Duration,
    pub retries: u32,
}

#[derive(Debug, Clone, Default)]
pub struct OutQueue {
    entries: Vec<OutPacketEntry>,
}

fn normalize_ack_type(packet_type: PacketType) -> PacketType {
    match packet_type {
        PacketType::Ack => PacketType::Command,
        PacketType::AckLow => PacketType::CommandLow,
        other => other,
    }
}

impl OutQueue {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    pub fn insert(
        &mut self,
        packet_type: PacketType,
        packet_id: u16,
        encoded_bytes: Vec<u8>,
        now: Instant,
        rto: Duration,
    ) {
        self.entries.push(OutPacketEntry {
            packet_id,
            packet_type,
            encoded_bytes,
            first_sent: now,
            last_sent: now,
            rto,
            retries: 0,
        });
    }

    pub fn insert_now(
        &mut self,
        packet_type: PacketType,
        packet_id: u16,
        encoded_bytes: Vec<u8>,
        rto: Duration,
    ) {
        self.insert(packet_type, packet_id, encoded_bytes, Instant::now(), rto);
    }

    pub fn acknowledge(&mut self, packet_type: PacketType, packet_id: u16) -> Option<Duration> {
        self.acknowledge_at(packet_type, packet_id, Instant::now())
    }

    pub fn acknowledge_at(
        &mut self,
        packet_type: PacketType,
        packet_id: u16,
        now: Instant,
    ) -> Option<Duration> {
        let norm_target = normalize_ack_type(packet_type);
        if let Some(pos) = self.entries.iter().position(|e| {
            normalize_ack_type(e.packet_type) == norm_target && e.packet_id == packet_id
        }) {
            let entry = self.entries.remove(pos);
            Some(now.saturating_duration_since(entry.last_sent))
        } else {
            None
        }
    }

    pub fn collect_resends(&mut self, now: Instant) -> Result<Vec<Vec<u8>>, ReliabilityError> {
        // Deterministic check for PACKET_TIMEOUT
        for entry in &self.entries {
            let elapsed = now.saturating_duration_since(entry.first_sent);
            if elapsed >= PACKET_TIMEOUT {
                return Err(ReliabilityError::PacketTimeout {
                    packet_type: entry.packet_type,
                    packet_id: entry.packet_id,
                    elapsed,
                });
            }
        }

        let mut resends = Vec::new();
        for entry in &mut self.entries {
            let elapsed_since_last = now.saturating_duration_since(entry.last_sent);
            if elapsed_since_last >= entry.rto {
                entry.retries += 1;
                entry.last_sent = now;
                entry.rto = (entry.rto.saturating_mul(2)).min(MAX_RETRY_INTERVAL);
                resends.push(entry.encoded_bytes.clone());
            }
        }
        Ok(resends)
    }

    pub fn clear_init1(&mut self) {
        self.entries.retain(|e| e.packet_type != PacketType::Init1);
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }

    pub fn entries(&self) -> &[OutPacketEntry] {
        &self.entries
    }
}

// ----------------------------------------------------------------------------
// 7. split_and_compress_c2s
// ----------------------------------------------------------------------------

pub fn split_and_compress_c2s(
    cmd_str: &str,
    packet_type: PacketType,
    new_protocol: bool,
) -> Vec<(PacketFlags, Vec<u8>)> {
    let raw = cmd_str.as_bytes();
    let mut add_flags = if new_protocol {
        PacketFlags::NEW_PROTOCOL
    } else {
        PacketFlags::NONE
    };

    let payload_buffer: Vec<u8>;
    let payload = if raw.len() > MAX_C2S_PAYLOAD && packet_type != PacketType::VoiceWhisper {
        let compressed = ts3_proto::quicklz::compress(raw);
        if compressed.len() < raw.len() {
            add_flags |= PacketFlags::COMPRESSED;
            payload_buffer = compressed;
            &payload_buffer[..]
        } else {
            raw
        }
    } else {
        raw
    };

    let chunks: Vec<&[u8]> = if payload.is_empty() {
        vec![&[]]
    } else {
        payload.chunks(MAX_C2S_PAYLOAD).collect()
    };

    let total_chunks = chunks.len();
    let mut result = Vec::with_capacity(total_chunks);

    for (i, &chunk) in chunks.iter().enumerate() {
        let is_first = i == 0;
        let is_last = i + 1 == total_chunks;
        let mut flags = PacketFlags::NONE;
        if is_first ^ is_last {
            flags |= PacketFlags::FRAGMENTED;
        }
        if is_first {
            flags |= add_flags;
        }
        result.push((flags, chunk.to_vec()));
    }

    result
}

// ----------------------------------------------------------------------------
// 8. CommandReassembler & SingleStreamReassembler
// ----------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemSetStatus {
    OutOfWindowNotSet,
    OutOfWindowSet,
    InWindowNotSet,
    InWindowSet,
}

impl ItemSetStatus {
    #[inline]
    pub fn is_set(&self) -> bool {
        matches!(self, Self::OutOfWindowSet | Self::InWindowSet)
    }

    #[inline]
    pub fn in_window(&self) -> bool {
        matches!(self, Self::InWindowNotSet | Self::InWindowSet)
    }

    #[inline]
    pub fn should_ack(&self) -> bool {
        !matches!(self, Self::OutOfWindowNotSet)
    }
}

#[derive(Debug, Clone)]
pub struct PacketFragment {
    pub flags: PacketFlags,
    pub data: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct SingleStreamReassembler {
    window: GenerationWindow,
    buffer: Vec<Option<PacketFragment>>,
    current_start: usize,
    capacity: usize,
}

pub type RingQueue = SingleStreamReassembler;

impl SingleStreamReassembler {
    pub fn new(window_size: u16) -> Self {
        let capacity = window_size as usize;
        Self {
            window: GenerationWindow::new(window_size),
            buffer: vec![None; capacity],
            current_start: 0,
            capacity,
        }
    }

    pub fn window(&self) -> &GenerationWindow {
        &self.window
    }

    pub fn get_generation(&self, packet_id: u16) -> Option<u32> {
        self.window.get_generation(packet_id)
    }

    pub fn generation_unchecked(&self, packet_id: u16) -> u32 {
        self.window.generation_unchecked(packet_id)
    }

    pub fn resolve_generation(&self, packet_id: u16) -> u32 {
        self.window.generation_unchecked(packet_id)
    }

    pub fn is_set(&self, packet_id: u16) -> ItemSetStatus {
        let index = self.window.mapped_to_index(packet_id);
        if index < 0 {
            ItemSetStatus::OutOfWindowSet
        } else if index >= self.capacity as isize {
            ItemSetStatus::OutOfWindowNotSet
        } else {
            let local = (self.current_start + index as usize) % self.capacity;
            if self.buffer[local].is_some() {
                ItemSetStatus::InWindowSet
            } else {
                ItemSetStatus::InWindowNotSet
            }
        }
    }

    pub fn slot_status(&self, packet_id: u16) -> ItemSetStatus {
        self.is_set(packet_id)
    }

    pub fn receive(
        &mut self,
        packet_id: u16,
        flags: PacketFlags,
        payload: Vec<u8>,
    ) -> Result<(bool, Vec<Vec<u8>>), ReliabilityError> {
        let status = self.is_set(packet_id);
        if status == ItemSetStatus::OutOfWindowNotSet {
            return Ok((false, Vec::new()));
        }

        // OutOfWindowSet or InWindowSet: ACK should be sent, but skip duplicate payload insertion
        if status.is_set() {
            return Ok((true, Vec::new()));
        }

        let index = self.window.mapped_to_index(packet_id);
        let local = (self.current_start + index as usize) % self.capacity;
        self.buffer[local] = Some(PacketFragment { flags, data: payload });

        let ready = self.drain_ready_commands()?;
        Ok((true, ready))
    }

    pub fn push_bytes(
        &mut self,
        packet_id: u16,
        flags: PacketFlags,
        payload: &[u8],
    ) -> Result<Vec<Vec<u8>>, ReliabilityError> {
        let (_, ready) = self.receive(packet_id, flags, payload.to_vec())?;
        Ok(ready)
    }

    pub fn push(
        &mut self,
        packet_id: u16,
        flags: PacketFlags,
        payload: &[u8],
    ) -> Result<Vec<String>, ReliabilityError> {
        let ready = self.push_bytes(packet_id, flags, payload)?;
        let mut strings = Vec::with_capacity(ready.len());
        for bytes in ready {
            strings.push(String::from_utf8(bytes)?);
        }
        Ok(strings)
    }

    pub fn push_str(
        &mut self,
        packet_id: u16,
        flags: PacketFlags,
        payload: &[u8],
    ) -> Result<Vec<String>, ReliabilityError> {
        self.push(packet_id, flags, payload)
    }

    fn drain_ready_commands(&mut self) -> Result<Vec<Vec<u8>>, ReliabilityError> {
        let mut results = Vec::new();

        loop {
            let mut take = 0;
            let mut has_start = false;
            let mut has_end = false;
            let mut is_compressed = false;

            for i in 0..self.capacity {
                let local = (self.current_start + i) % self.capacity;
                if let Some(frag) = &self.buffer[local] {
                    take += 1;
                    if i == 0 {
                        is_compressed = frag.flags.contains(PacketFlags::COMPRESSED);
                    }
                    if frag.flags.contains(PacketFlags::FRAGMENTED) {
                        if !has_start {
                            has_start = true;
                        } else {
                            has_end = true;
                            break;
                        }
                    } else {
                        if !has_start {
                            has_start = true;
                            has_end = true;
                            break;
                        }
                    }
                } else {
                    break;
                }
            }

            if !has_start || !has_end {
                break;
            }

            let mut merged = Vec::new();
            for _ in 0..take {
                let frag = self.buffer[self.current_start]
                    .take()
                    .expect("packet fragment must exist");
                merged.extend_from_slice(&frag.data);
                self.current_start = (self.current_start + 1) % self.capacity;
                self.window.advance(1);
            }

            let final_data = if is_compressed {
                ts3_proto::quicklz::decompress(&merged, MAX_DECOMPRESSED_SIZE)?
            } else {
                merged
            };

            results.push(final_data);
        }

        Ok(results)
    }

    pub fn clear(&mut self) {
        self.buffer.fill(None);
        self.current_start = 0;
        self.window.reset();
    }

    pub fn reset(&mut self) {
        self.clear();
    }
}

impl Default for SingleStreamReassembler {
    fn default() -> Self {
        Self::new(COMMAND_WINDOW_SIZE)
    }
}

#[derive(Debug, Clone)]
pub struct CommandReassembler {
    pub command: SingleStreamReassembler,
    pub command_low: SingleStreamReassembler,
}

impl CommandReassembler {
    pub fn new() -> Self {
        Self {
            command: SingleStreamReassembler::new(COMMAND_WINDOW_SIZE),
            command_low: SingleStreamReassembler::new(COMMAND_WINDOW_SIZE),
        }
    }

    pub fn get_generation(&self, packet_type: PacketType, packet_id: u16) -> Option<u32> {
        match packet_type {
            PacketType::Command => self.command.get_generation(packet_id),
            PacketType::CommandLow => self.command_low.get_generation(packet_id),
            _ => None,
        }
    }

    pub fn resolve_generation(&self, packet_id: u16) -> u32 {
        self.command.resolve_generation(packet_id)
    }

    pub fn slot_status(&self, packet_id: u16) -> ItemSetStatus {
        self.command.is_set(packet_id)
    }

    pub fn receive(
        &mut self,
        packet_type: PacketType,
        packet_id: u16,
        flags: PacketFlags,
        payload: Vec<u8>,
    ) -> Result<(bool, Vec<Vec<u8>>), ReliabilityError> {
        match packet_type {
            PacketType::Command => self.command.receive(packet_id, flags, payload),
            PacketType::CommandLow => self.command_low.receive(packet_id, flags, payload),
            _ => Ok((false, Vec::new())),
        }
    }

    pub fn push_bytes(
        &mut self,
        packet_id: u16,
        flags: PacketFlags,
        payload: &[u8],
    ) -> Result<Vec<Vec<u8>>, ReliabilityError> {
        self.command.push_bytes(packet_id, flags, payload)
    }

    pub fn push(
        &mut self,
        packet_id: u16,
        flags: PacketFlags,
        payload: &[u8],
    ) -> Result<Vec<String>, ReliabilityError> {
        self.command.push(packet_id, flags, payload)
    }

    pub fn push_str(
        &mut self,
        packet_id: u16,
        flags: PacketFlags,
        payload: &[u8],
    ) -> Result<Vec<String>, ReliabilityError> {
        self.command.push_str(packet_id, flags, payload)
    }

    pub fn push_typed(
        &mut self,
        packet_type: PacketType,
        packet_id: u16,
        flags: PacketFlags,
        payload: &[u8],
    ) -> Result<Vec<Vec<u8>>, ReliabilityError> {
        let (_, ready) = self.receive(packet_type, packet_id, flags, payload.to_vec())?;
        Ok(ready)
    }

    pub fn push_typed_str(
        &mut self,
        packet_type: PacketType,
        packet_id: u16,
        flags: PacketFlags,
        payload: &[u8],
    ) -> Result<Vec<String>, ReliabilityError> {
        let ready = self.push_typed(packet_type, packet_id, flags, payload)?;
        let mut strings = Vec::with_capacity(ready.len());
        for bytes in ready {
            strings.push(String::from_utf8(bytes)?);
        }
        Ok(strings)
    }

    pub fn clear(&mut self) {
        self.command.clear();
        self.command_low.clear();
    }

    pub fn reset(&mut self) {
        self.clear();
    }
}

impl Default for CommandReassembler {
    fn default() -> Self {
        Self::new()
    }
}

// ----------------------------------------------------------------------------
// 9. Unit Tests
// ----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generation_window_wraparound_and_advance() {
        let mut win = GenerationWindow::command(); // 128
        assert_eq!(win.window_size, 128);
        assert_eq!(win.mapped_base_offset, 0);
        assert_eq!(win.generation, 0);

        // Advance close to 65535 wraparound boundary
        win.advance(65534);
        assert_eq!(win.mapped_base_offset, 65534);
        assert_eq!(win.generation, 0);

        // Window spans [65534 .. 65535, 0 .. 125]
        assert!(win.is_in_window(65534));
        assert!(win.is_in_window(65535));
        assert!(win.is_in_window(0));
        assert!(win.is_in_window(125));
        assert!(!win.is_in_window(126));
        assert!(!win.is_in_window(65533));

        // Next generation check: 0 is in next gen, 65535 is in current gen
        assert!(!win.is_next_gen(65535));
        assert!(win.is_next_gen(0));
        assert!(win.is_next_gen(100));

        // Generation values
        assert_eq!(win.get_generation(65534), Some(0));
        assert_eq!(win.get_generation(65535), Some(0));
        assert_eq!(win.get_generation(0), Some(1));
        assert_eq!(win.get_generation(125), Some(1));
        assert_eq!(win.get_generation(126), None);

        // Index mapping
        assert_eq!(win.mapped_to_index(65534), 0);
        assert_eq!(win.mapped_to_index(65535), 1);
        assert_eq!(win.mapped_to_index(0), 2);
        assert_eq!(win.mapped_to_index(1), 3);

        // accept_and_advance to packet 0: excluded packet is 0 (index 2), so advances by 3
        let gen = win.accept_and_advance(0);
        assert_eq!(gen, Some(1));
        // 65534 + 3 = 65537 => base = 1, generation = 1
        assert_eq!(win.mapped_base_offset, 1);
        assert_eq!(win.generation, 1);

        // Voice vs command window sizes
        let voice_win = GenerationWindow::voice();
        assert_eq!(voice_win.window_size, 32768);
        assert_eq!(GenerationWindow::new_voice().window_size, 32768);
        let cmd_win = GenerationWindow::command();
        assert_eq!(cmd_win.window_size, 128);
        assert_eq!(GenerationWindow::new_command().window_size, 128);
    }

    #[test]
    fn test_out_sequence_tracker() {
        let mut tracker = OutSequenceTracker::new();

        // Init1 always returns (101, 0) without advancing
        assert_eq!(tracker.next(PacketType::Init1), (101, 0));
        assert_eq!(tracker.next(PacketType::Init1), (101, 0));
        assert_eq!(tracker.peek(PacketType::Init1), (101, 0));

        // Initial Command is 0
        assert_eq!(tracker.peek(PacketType::Command), (0, 0));
        // skip_command_id_zero advances Command from 0 to 1
        tracker.skip_command_id_zero();
        assert_eq!(tracker.peek(PacketType::Command), (1, 0));
        assert_eq!(tracker.next(PacketType::Command), (1, 0));
        assert_eq!(tracker.peek(PacketType::Command), (2, 0));

        // Test wraparound at 65535 -> 0 incrementing generation
        let idx = PacketType::Voice as usize;
        tracker.packet_counter[idx] = 65535;
        tracker.generation_counter[idx] = 0;

        assert_eq!(tracker.next(PacketType::Voice), (65535, 0));
        assert_eq!(tracker.next(PacketType::Voice), (0, 1));
        assert_eq!(tracker.next(PacketType::Voice), (1, 1));
    }

    #[test]
    fn test_rtt_estimator() {
        let mut rtt = RttEstimator::new();
        assert_eq!(rtt.srtt(), MAX_RETRY_INTERVAL);
        assert_eq!(rtt.srtt_or_default(), MAX_RETRY_INTERVAL);
        assert_eq!(rtt.rto(), Duration::from_millis(1000));
        assert_eq!(rtt.rttvar(), Duration::ZERO);

        // First sample: 50ms
        rtt.update(Duration::from_millis(50));
        assert_eq!(rtt.srtt(), Duration::from_millis(50));
        assert_eq!(rtt.rttvar(), Duration::ZERO);
        // var_term = max(100ms, 4 * 0) = 100ms floor
        // rto = 50ms + 100ms = 150ms
        assert_eq!(rtt.rto(), Duration::from_millis(150));

        // Second sample: 70ms
        // srtt = 0.875 * 50ms + 0.125 * 70ms = 43.75 + 8.75 = 52.5ms
        // diff = |70 - 52.5| = 17.5ms
        // rttvar = 0.75 * 0 + 0.25 * 17.5 = 4.375ms
        // 4 * rttvar = 17.5ms < 100ms => var_term = 100ms
        // rto = 52.5ms + 100ms = 152.5ms
        rtt.update(Duration::from_millis(70));
        assert_eq!(rtt.srtt(), Duration::from_secs_f64(0.0525));
        assert_eq!(rtt.rttvar(), Duration::from_secs_f64(0.004375));
        assert_eq!(rtt.rto(), Duration::from_secs_f64(0.1525));

        // Exponential backoff capped at MAX_RETRY_INTERVAL (1000ms)
        let backed1 = rtt.backoff();
        assert_eq!(backed1, Duration::from_secs_f64(0.305));
        let backed2 = rtt.backoff();
        assert_eq!(backed2, Duration::from_secs_f64(0.610));
        let backed3 = rtt.backoff();
        assert_eq!(backed3, Duration::from_millis(1000));
        let backed4 = rtt.backoff();
        assert_eq!(backed4, Duration::from_millis(1000));
    }

    #[test]
    fn test_out_queue() {
        let mut queue = OutQueue::new();
        let t0 = Instant::now();

        // Insert command 1, command_low 1, and init1
        queue.insert(
            PacketType::Command,
            1,
            vec![1, 2, 3],
            t0,
            Duration::from_millis(200),
        );
        queue.insert(
            PacketType::CommandLow,
            1,
            vec![4, 5, 6],
            t0,
            Duration::from_millis(200),
        );
        queue.insert(
            PacketType::Init1,
            101,
            vec![7, 8, 9],
            t0,
            Duration::from_millis(200),
        );
        assert_eq!(queue.len(), 3);

        // Clear init1
        queue.clear_init1();
        assert_eq!(queue.len(), 2);

        // Acknowledge command 1 via PacketType::Ack normalized matching
        let sample = queue.acknowledge_at(PacketType::Ack, 1, t0 + Duration::from_millis(45));
        assert_eq!(sample, Some(Duration::from_millis(45)));
        assert_eq!(queue.len(), 1);

        // Resend check: at t0 + 100ms, not timed out for resend yet (rto is 200ms)
        let resends = queue
            .collect_resends(t0 + Duration::from_millis(100))
            .unwrap();
        assert!(resends.is_empty());

        // At t0 + 250ms: rto expired!
        let resends = queue
            .collect_resends(t0 + Duration::from_millis(250))
            .unwrap();
        assert_eq!(resends.len(), 1);
        assert_eq!(resends[0], vec![4, 5, 6]);

        // Retries incremented and RTO backed off to 400ms
        let entry = &queue.entries()[0];
        assert_eq!(entry.retries, 1);
        assert_eq!(entry.rto, Duration::from_millis(400));
        assert_eq!(entry.last_sent, t0 + Duration::from_millis(250));

        // 30s timeout check
        let timeout_err = queue
            .collect_resends(t0 + Duration::from_secs(31))
            .unwrap_err();
        match timeout_err {
            ReliabilityError::PacketTimeout {
                packet_type,
                packet_id,
                elapsed,
            } => {
                assert_eq!(packet_type, PacketType::CommandLow);
                assert_eq!(packet_id, 1);
                assert!(elapsed >= Duration::from_secs(30));
            }
            _ => panic!("unexpected error: {:?}", timeout_err),
        }
    }

    #[test]
    fn test_split_and_compress_c2s_and_reassembly() {
        // Create large command with varying values so compressed size still exceeds MAX_C2S_PAYLOAD (487)
        let mut original_cmd = String::new();
        for i in 0..150 {
            original_cmd.push_str(&format!(
                "clientvariable_{}=sample_value_channel_info_test_payload_with_unique_index_{}_and_hash_{}&",
                i,
                i * 73 + 19,
                (i * 104729) ^ 0x5a5a5a
            ));
        }

        let chunks = split_and_compress_c2s(&original_cmd, PacketType::Command, true);
        assert!(chunks.len() >= 2);

        let total = chunks.len();
        for (i, (flags, payload)) in chunks.iter().enumerate() {
            assert!(payload.len() <= MAX_C2S_PAYLOAD);
            if i == 0 {
                // First chunk has FRAGMENTED, COMPRESSED, and NEW_PROTOCOL
                assert!(flags.contains(PacketFlags::FRAGMENTED));
                assert!(flags.contains(PacketFlags::COMPRESSED));
                assert!(flags.contains(PacketFlags::NEW_PROTOCOL));
            } else if i + 1 == total {
                // Last chunk has FRAGMENTED
                assert!(flags.contains(PacketFlags::FRAGMENTED));
                assert!(!flags.contains(PacketFlags::COMPRESSED));
                assert!(!flags.contains(PacketFlags::NEW_PROTOCOL));
            } else {
                // Middle chunks have NONE
                assert_eq!(*flags, PacketFlags::NONE);
            }
        }

        // Test out-of-order and duplicate arrival reassembly
        let mut reassembler = CommandReassembler::new();

        // Feed last chunk first
        let last_idx = (total - 1) as u16;
        let (ack_last, res_last) = reassembler
            .receive(
                PacketType::Command,
                last_idx,
                chunks[total - 1].0,
                chunks[total - 1].1.clone(),
            )
            .unwrap();
        assert!(ack_last);
        assert!(res_last.is_empty());

        // Duplicate of last chunk: should ack, but drop duplicate
        let (ack_dup, res_dup) = reassembler
            .receive(
                PacketType::Command,
                last_idx,
                chunks[total - 1].0,
                chunks[total - 1].1.clone(),
            )
            .unwrap();
        assert!(ack_dup);
        assert!(res_dup.is_empty());

        // Feed intermediate chunks if total > 2
        for i in (1..(total - 1)).rev() {
            let (ack, res) = reassembler
                .receive(
                    PacketType::Command,
                    i as u16,
                    chunks[i].0,
                    chunks[i].1.clone(),
                )
                .unwrap();
            assert!(ack);
            assert!(res.is_empty());
        }

        // Finally feed first chunk (packet 0)
        let (ack_first, res_first) = reassembler
            .receive(
                PacketType::Command,
                0,
                chunks[0].0,
                chunks[0].1.clone(),
            )
            .unwrap();
        assert!(ack_first);
        assert_eq!(res_first.len(), 1);

        let reassembled_str = String::from_utf8(res_first[0].clone()).unwrap();
        assert_eq!(reassembled_str, original_cmd);
    }
}
