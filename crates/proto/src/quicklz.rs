// Owned by Pool Agent 2 (arthoroy26a@gmail.com)

const TABLE_SIZE: usize = 4096;
const SET_CONTROL: u32 = 0x8000_0000;

/// Maximum allowed decompressed payload size (1 MiB, matching `PacketHandlerConst.MaxDecompressedSize`).
pub const MAX_DECOMPRESSED_SIZE: usize = 1024 * 1024;

/// Errors that can occur during QuickLZ header parsing or decompression.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum QuickLzError {
    #[error("truncated QuickLZ header")]
    TruncatedHeader,
    #[error("decompressed size {0} exceeds maximum allowed limit")]
    DecompressedLimitExceeded(usize),
    #[error("corrupted QuickLZ stream")]
    CorruptedStream,
    #[error("unsupported QuickLZ compression level")]
    UnsupportedLevel,
}

/// Returns the QuickLZ header length (3 or 9 bytes), validating that `source` is long enough to contain it.
#[inline]
pub fn header_len(source: &[u8]) -> Result<usize, QuickLzError> {
    if source.is_empty() {
        return Err(QuickLzError::TruncatedHeader);
    }
    let len = if (source[0] & 0x02) == 0 { 3 } else { 9 };
    if source.len() < len {
        return Err(QuickLzError::TruncatedHeader);
    }
    Ok(len)
}

/// Reads the decompressed size from a QuickLZ header.
#[inline]
pub fn size_decompressed(source: &[u8]) -> Result<usize, QuickLzError> {
    let h_len = header_len(source)?;
    if h_len == 3 {
        Ok(source[2] as usize)
    } else {
        Ok(u32::from_le_bytes([source[5], source[6], source[7], source[8]]) as usize)
    }
}

/// Reads the total compressed size (including header) from a QuickLZ header.
#[inline]
pub fn size_compressed(source: &[u8]) -> Result<usize, QuickLzError> {
    let h_len = header_len(source)?;
    if h_len == 3 {
        Ok(source[1] as usize)
    } else {
        Ok(u32::from_le_bytes([source[1], source[2], source[3], source[4]]) as usize)
    }
}

#[inline]
fn hash_fn(val: u32) -> usize {
    (((val >> 12) ^ val) & 0x0FFF) as usize
}

#[inline]
fn read_u24(buf: &[u8], off: usize) -> u32 {
    (buf[off] as u32) | ((buf[off + 1] as u32) << 8) | ((buf[off + 2] as u32) << 16)
}

#[inline]
fn write_u24(buf: &mut [u8], off: usize, val: u32) {
    buf[off] = (val & 0xFF) as u8;
    buf[off + 1] = ((val >> 8) & 0xFF) as u8;
    buf[off + 2] = ((val >> 16) & 0xFF) as u8;
}

#[inline]
fn is_6_same(slice: &[u8]) -> bool {
    let i0 = u32::from_le_bytes([slice[0], slice[1], slice[2], slice[3]]);
    let u1 = u16::from_le_bytes([slice[4], slice[5]]);
    (i0 == (i0 >> 8)) && ((i0 as u16) == u1)
}

#[inline]
fn write_header(dest: &mut [u8], src_len: usize, level: u8, h_len: usize, compressed: bool) {
    let mut flags = (level << 2) | 0x40;
    if compressed {
        flags |= 0x01;
    }

    if h_len == 3 {
        dest[0] = flags;
        dest[1] = dest.len() as u8;
        dest[2] = src_len as u8;
    } else {
        dest[0] = flags | 0x02;
        let comp_len = (dest.len() as u32).to_le_bytes();
        let decomp_len = (src_len as u32).to_le_bytes();
        dest[1..5].copy_from_slice(&comp_len);
        dest[5..9].copy_from_slice(&decomp_len);
    }
}

/// Decompresses a QuickLZ 1.5.0 level-1 payload, enforcing `max_size` on the decompressed output.
pub fn decompress(source: &[u8], max_size: usize) -> Result<Vec<u8>, QuickLzError> {
    let h_len = header_len(source)?;
    let flags = source[0];
    let level = (flags >> 2) & 0b11;
    if level != 1 {
        return Err(QuickLzError::UnsupportedLevel);
    }

    let compressed_size = size_compressed(source)?;
    let decompressed_size = size_decompressed(source)?;

    if decompressed_size > max_size {
        return Err(QuickLzError::DecompressedLimitExceeded(decompressed_size));
    }

    if source.len() < compressed_size || compressed_size < h_len {
        return Err(QuickLzError::CorruptedStream);
    }

    let src = &source[..compressed_size];

    if (flags & 0x01) == 0 {
        if compressed_size - h_len != decompressed_size {
            return Err(QuickLzError::CorruptedStream);
        }
        return Ok(src[h_len..h_len + decompressed_size].to_vec());
    }

    let mut dest = vec![0u8; decompressed_size];
    let mut control: u32 = 1;
    let mut src_pos = h_len;
    let mut dest_pos = 0usize;
    let mut next_hashed = 0usize;
    let mut hashtable = [0usize; TABLE_SIZE];

    loop {
        if control == 1 {
            if src_pos + 4 > src.len() {
                return Err(QuickLzError::CorruptedStream);
            }
            control = u32::from_le_bytes([
                src[src_pos],
                src[src_pos + 1],
                src[src_pos + 2],
                src[src_pos + 3],
            ]);
            src_pos += 4;
        }

        if (control & 1) != 0 {
            control >>= 1;
            if src_pos + 2 > src.len() {
                return Err(QuickLzError::CorruptedStream);
            }
            let next = src[src_pos];
            let b1 = src[src_pos + 1];
            src_pos += 2;

            let hash = ((next as usize) >> 4) | ((b1 as usize) << 4);
            let mut match_len = (next & 0x0F) as usize;
            if match_len != 0 {
                match_len += 2;
            } else {
                if src_pos >= src.len() {
                    return Err(QuickLzError::CorruptedStream);
                }
                match_len = src[src_pos] as usize;
                src_pos += 1;
                if match_len < 3 {
                    return Err(QuickLzError::CorruptedStream);
                }
            }

            let offset = hashtable[hash];
            let new_dest_pos = dest_pos
                .checked_add(match_len)
                .ok_or(QuickLzError::CorruptedStream)?;
            if offset >= dest_pos || new_dest_pos > decompressed_size {
                return Err(QuickLzError::CorruptedStream);
            }

            for i in 0..match_len {
                dest[dest_pos + i] = dest[offset + i];
            }
            dest_pos = new_dest_pos;

            let end = dest_pos + 1 - match_len;
            for i in next_hashed..end {
                if i + 3 <= dest.len() {
                    let v = read_u24(&dest, i);
                    hashtable[hash_fn(v)] = i;
                }
            }
            next_hashed = dest_pos;
        } else if dest_pos >= decompressed_size.saturating_sub(10) {
            while dest_pos < decompressed_size {
                if control == 1 {
                    if src_pos + 4 > src.len() {
                        return Err(QuickLzError::CorruptedStream);
                    }
                    src_pos += 4;
                }
                control >>= 1;
                if src_pos >= src.len() {
                    return Err(QuickLzError::CorruptedStream);
                }
                dest[dest_pos] = src[src_pos];
                dest_pos += 1;
                src_pos += 1;
            }
            break;
        } else {
            if src_pos >= src.len() || dest_pos >= decompressed_size {
                return Err(QuickLzError::CorruptedStream);
            }
            dest[dest_pos] = src[src_pos];
            dest_pos += 1;
            src_pos += 1;
            control >>= 1;

            let end = dest_pos.saturating_sub(2);
            for i in next_hashed..end {
                if i + 3 <= dest.len() {
                    let v = read_u24(&dest, i);
                    hashtable[hash_fn(v)] = i;
                }
            }
            next_hashed = next_hashed.max(end);
        }
    }

    Ok(dest)
}

/// Compresses `source` using QuickLZ 1.5.0 level-1 compression.
pub fn compress(source: &[u8]) -> Vec<u8> {
    let h_len = if source.len() < 216 { 3 } else { 9 };
    let mut dest = vec![0u8; source.len() + 400];
    let mut dest_pos = h_len + 4;
    let mut control = SET_CONTROL;
    let mut control_pos = h_len;
    let mut source_pos = 0usize;

    let mut unmatched = 0usize;
    let mut hashtable = [0usize; TABLE_SIZE];
    let mut hash_counter = [false; TABLE_SIZE];
    let mut cachetable = [0u32; TABLE_SIZE];

    let source_limit = source.len().saturating_sub(10);
    while source_pos < source_limit {
        if (control & 1) != 0 {
            if source_pos > source.len() / 2 && dest_pos > source_pos - (source_pos / 32) {
                let total_len = h_len + source.len();
                dest[h_len..total_len].copy_from_slice(source);
                dest.truncate(total_len);
                write_header(&mut dest, source.len(), 1, h_len, false);
                return dest;
            }
            let word = ((control >> 1) | SET_CONTROL).to_le_bytes();
            dest[control_pos..control_pos + 4].copy_from_slice(&word);
            control_pos = dest_pos;
            dest_pos += 4;
            control = SET_CONTROL;
        }

        let next = read_u24(source, source_pos);
        let hash = hash_fn(next);
        let offset = hashtable[hash];
        let cache = cachetable[hash];
        cachetable[hash] = next;
        hashtable[hash] = source_pos;

        let is_match = cache == next
            && hash_counter[hash]
            && (source_pos - offset >= 3
                || (source_pos == offset + 1
                    && unmatched >= 3
                    && source_pos > 3
                    && is_6_same(&source[source_pos - 3..source_pos + 3])));

        if is_match {
            control = (control >> 1) | SET_CONTROL;
            let mut match_len = 3usize;
            let remainder = (source.len() - 4 - source_pos).min(0xFF);
            while source[offset + match_len] == source[source_pos + match_len]
                && match_len < remainder
            {
                match_len += 1;
            }

            if match_len < 18 {
                let token = ((hash << 4) | (match_len - 2)) as u16;
                dest[dest_pos..dest_pos + 2].copy_from_slice(&token.to_le_bytes());
                dest_pos += 2;
            } else {
                let token = ((hash as u32) << 4) | ((match_len as u32) << 16);
                write_u24(&mut dest, dest_pos, token);
                dest_pos += 3;
            }
            source_pos += match_len;
            unmatched = 0;
        } else {
            unmatched += 1;
            hash_counter[hash] = true;
            dest[dest_pos] = source[source_pos];
            dest_pos += 1;
            source_pos += 1;
            control >>= 1;
        }
    }

    while source_pos < source.len() {
        if (control & 1) != 0 {
            let word = ((control >> 1) | SET_CONTROL).to_le_bytes();
            dest[control_pos..control_pos + 4].copy_from_slice(&word);
            control_pos = dest_pos;
            dest_pos += 4;
            control = SET_CONTROL;
        }
        dest[dest_pos] = source[source_pos];
        dest_pos += 1;
        source_pos += 1;
        control >>= 1;
    }

    while (control & 1) == 0 {
        control >>= 1;
    }
    let word = ((control >> 1) | SET_CONTROL).to_le_bytes();
    dest[control_pos..control_pos + 4].copy_from_slice(&word);

    dest.truncate(dest_pos);
    write_header(&mut dest, source.len(), 1, h_len, true);
    dest
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_short_payloads_roundtrip() {
        for &len in &[0usize, 1, 10, 100, 215] {
            let payload: Vec<u8> = (0..len).map(|i| (i * 7 + 3) as u8).collect();
            let compressed = compress(&payload);
            assert_eq!(header_len(&compressed).unwrap(), 3);
            assert_eq!(size_decompressed(&compressed).unwrap(), len);
            assert_eq!(size_compressed(&compressed).unwrap(), compressed.len());

            let decompressed = decompress(&compressed, MAX_DECOMPRESSED_SIZE).unwrap();
            assert_eq!(decompressed, payload);
        }
    }

    #[test]
    fn test_long_payloads_roundtrip() {
        for &len in &[216usize, 1000, 10_000] {
            let payload: Vec<u8> = (0..len).map(|i| ((i % 29) as u8).wrapping_add(b'a')).collect();
            let compressed = compress(&payload);
            assert_eq!(header_len(&compressed).unwrap(), 9);
            assert_eq!(size_decompressed(&compressed).unwrap(), len);
            assert_eq!(size_compressed(&compressed).unwrap(), compressed.len());

            let decompressed = decompress(&compressed, MAX_DECOMPRESSED_SIZE).unwrap();
            assert_eq!(decompressed, payload);
        }
    }

    #[test]
    fn test_repetitive_ts3_commands() {
        let channellist = concat!(
            "channellist cid=1 pid=0 channel_order=0 channel_name=Default\\sChannel channel_topic= ",
            "channel_flag_default=1 channel_flag_password=0 channel_flag_permanent=1 channel_flag_semi_permanent=0 ",
            "channel_codec=4 channel_codec_quality=6 channel_needed_talk_power=0|",
            "cid=2 pid=0 channel_order=1 channel_name=Channel\\s2 channel_topic= ",
            "channel_flag_default=0 channel_flag_password=0 channel_flag_permanent=1 channel_flag_semi_permanent=0 ",
            "channel_codec=4 channel_codec_quality=6 channel_needed_talk_power=0|",
            "cid=3 pid=0 channel_order=2 channel_name=Channel\\s3 channel_topic= ",
            "channel_flag_default=0 channel_flag_password=0 channel_flag_permanent=1 channel_flag_semi_permanent=0 ",
            "channel_codec=4 channel_codec_quality=6 channel_needed_talk_power=0"
        )
        .as_bytes();

        let comp_chan = compress(channellist);
        assert!(comp_chan.len() < channellist.len());
        assert_eq!(
            decompress(&comp_chan, MAX_DECOMPRESSED_SIZE).unwrap(),
            channellist
        );

        let notify_enter = concat!(
            "notifycliententerview cfid=0 ctid=1 reasonid=0 clid=12 client_unique_identifier=abc123xyz= ",
            "client_nickname=TeamSpeakUser1 client_input_muted=0 client_output_muted=0 client_outputonly_muted=0 ",
            "client_input_hardware=1 client_output_hardware=1 client_meta_data client_is_recording=0 ",
            "client_database_id=42 client_channel_group_id=8 client_servergroups=6,7 client_away=0 client_away_message= ",
            "client_type=0 client_talk_power=75 client_talk_request=0 client_talk_request_msg= ",
            "client_description= client_is_talker=0 client_is_priority_speaker=0 client_unread_messages=0 ",
            "client_nickname_phonetic= client_needed_serverquery_view_power=75 client_icon_id=0 client_is_channel_commander=0 ",
            "client_country=DE client_channel_group_inherited_channel_id=1 client_badges="
        )
        .as_bytes();

        let comp_enter = compress(notify_enter);
        assert!(comp_enter.len() < notify_enter.len());
        assert_eq!(
            decompress(&comp_enter, MAX_DECOMPRESSED_SIZE).unwrap(),
            notify_enter
        );

        // Also test run-length (Is6Same with zero bytes) & long match (>= 18 bytes)
        let mut rle_payload = vec![0u8; 500];
        rle_payload.extend_from_slice(notify_enter);
        let comp_rle = compress(&rle_payload);
        assert!(comp_rle.len() < rle_payload.len());
        assert_eq!(
            decompress(&comp_rle, MAX_DECOMPRESSED_SIZE).unwrap(),
            rle_payload
        );
    }

    #[test]
    fn test_incompressible_fallback() {
        // Pseudo-random LCG stream that cannot be compressed
        let make_random = |len: usize| -> Vec<u8> {
            let mut state: u64 = 0xDEAD_BEEF_CAFE_BABEu64;
            (0..len)
                .map(|_| {
                    state = state
                        .wrapping_mul(6364136223846793005)
                        .wrapping_add(1442695040888963407);
                    (state >> 32) as u8
                })
                .collect()
        };

        // > 64B short header incompressible
        let buf_short = make_random(120);
        let comp_short = compress(&buf_short);
        assert_eq!(comp_short[0] & 0x01, 0, "expected uncompressed flag for 120B random buffer");
        assert_eq!(header_len(&comp_short).unwrap(), 3);
        assert_eq!(comp_short.len(), 3 + buf_short.len());
        assert_eq!(
            decompress(&comp_short, MAX_DECOMPRESSED_SIZE).unwrap(),
            buf_short
        );

        // > 300B long header incompressible
        let buf_long = make_random(400);
        let comp_long = compress(&buf_long);
        assert_eq!(comp_long[0] & 0x01, 0, "expected uncompressed flag for 400B random buffer");
        assert_eq!(header_len(&comp_long).unwrap(), 9);
        assert_eq!(comp_long.len(), 9 + buf_long.len());
        assert_eq!(
            decompress(&comp_long, MAX_DECOMPRESSED_SIZE).unwrap(),
            buf_long
        );
    }

    #[test]
    fn test_corrupted_inputs_bounds_safety() {
        // 1. Empty slice
        assert_eq!(header_len(&[]), Err(QuickLzError::TruncatedHeader));
        assert_eq!(size_compressed(&[]), Err(QuickLzError::TruncatedHeader));
        assert_eq!(size_decompressed(&[]), Err(QuickLzError::TruncatedHeader));
        assert_eq!(
            decompress(&[], MAX_DECOMPRESSED_SIZE),
            Err(QuickLzError::TruncatedHeader)
        );

        // 2. Truncated 3B header
        assert_eq!(
            decompress(&[0x45, 0x10], MAX_DECOMPRESSED_SIZE),
            Err(QuickLzError::TruncatedHeader)
        );

        // 3. Truncated 9B header (bit 1 set: 0x47)
        assert_eq!(
            decompress(&[0x47, 1, 2, 3, 4, 5, 6, 7], MAX_DECOMPRESSED_SIZE),
            Err(QuickLzError::TruncatedHeader)
        );

        // 4. Unsupported level (level 3 -> (3 << 2) | 0x41 = 0x4D)
        assert_eq!(
            decompress(&[0x4D, 3, 0], MAX_DECOMPRESSED_SIZE),
            Err(QuickLzError::UnsupportedLevel)
        );

        // 5. DecompressedLimitExceeded
        let valid_comp = compress(b"hello world this is a test payload");
        assert_eq!(
            decompress(&valid_comp, 10),
            Err(QuickLzError::DecompressedLimitExceeded(34))
        );

        // 6. Truncated compressed stream (compressed_size > source.len())
        assert_eq!(
            decompress(&valid_comp[..valid_comp.len() - 1], MAX_DECOMPRESSED_SIZE),
            Err(QuickLzError::CorruptedStream)
        );

        // 7. Corrupted uncompressed size mismatch ((flags & 1) == 0, comp_size - h_len != decomp_size)
        let bad_uncomp = [0x44u8, 10, 8, 1, 2, 3, 4, 5, 6, 7];
        assert_eq!(
            decompress(&bad_uncomp, MAX_DECOMPRESSED_SIZE),
            Err(QuickLzError::CorruptedStream)
        );

        // 8. Out-of-bounds match offset (first token is a match at dest_pos == 0)
        // Header: 3B, compressed_size = 9, decompressed_size = 20
        // Control word: 0x8000_0001 (bit 0 = 1 -> match immediately)
        // Match token: 2 bytes (hash = 0, len = 3 -> 0x01, 0x00)
        let oob_match = [0x45u8, 9, 20, 0x01, 0x00, 0x00, 0x80, 0x01, 0x00];
        assert_eq!(
            decompress(&oob_match, MAX_DECOMPRESSED_SIZE),
            Err(QuickLzError::CorruptedStream)
        );

        // 9. Truncated match token (control word has match bit set, but only 1 byte follows)
        let trunc_match = [0x45u8, 8, 20, 0x01, 0x00, 0x00, 0x80, 0x01];
        assert_eq!(
            decompress(&trunc_match, MAX_DECOMPRESSED_SIZE),
            Err(QuickLzError::CorruptedStream)
        );
    }
}
