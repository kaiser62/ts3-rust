use num_bigint::BigUint;
use num_traits::Zero;

/// Output byte length of the TeamSpeak 3 RSA handshake puzzle solution (`512` bits = `64` bytes).
pub const RSA_PUZZLE_LEN: usize = 64;

/// Maximum valid puzzle difficulty level accepted by the TeamSpeak 3 handshake (`TsCrypt.cs:497`).
pub const MAX_RSA_PUZZLE_LEVEL: u32 = 1_000_000;

/// Computes the TeamSpeak 3 `TS3INIT1` step 3/4 RSA time-lock puzzle solution:
/// `y = x^(2^level) mod n` via `level` repeated modular squarings.
///
/// Returns a 64-byte big-endian `Vec<u8>`, left-padded with `0x00` to 64 bytes
/// (or the last 64 bytes if the modulus exceeds 64 bytes), matching `TsCrypt.cs:474, 495-504`.
/// If `n == 0`, returns 64 zero bytes.
pub fn solve_rsa_puzzle(x_be: &[u8], n_be: &[u8], level: u32) -> Vec<u8> {
    let n = BigUint::from_bytes_be(n_be);
    if n.is_zero() {
        return vec![0u8; RSA_PUZZLE_LEN];
    }

    let x = BigUint::from_bytes_be(x_be);
    let mut y = x % &n;
    for _ in 0..level {
        y = (&y * &y) % &n;
    }

    let raw = y.to_bytes_be();
    if raw.len() == RSA_PUZZLE_LEN {
        raw
    } else if raw.len() < RSA_PUZZLE_LEN {
        let mut padded = vec![0u8; RSA_PUZZLE_LEN];
        padded[RSA_PUZZLE_LEN - raw.len()..].copy_from_slice(&raw);
        padded
    } else {
        raw[raw.len() - RSA_PUZZLE_LEN..].to_vec()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_solve_rsa_puzzle_small_values() {
        // x = 2, n = 17, level = 3 => 2^(2^3) mod 17 = 2^8 mod 17 = 256 mod 17 = 1
        let y = solve_rsa_puzzle(&[2], &[17], 3);
        assert_eq!(y.len(), 64);
        assert!(y[..63].iter().all(|&b| b == 0));
        assert_eq!(y[63], 1);

        // x = 3, n = 100, level = 0 => 3^(2^0) mod 100 = 3
        let y0 = solve_rsa_puzzle(&[3], &[100], 0);
        assert_eq!(y0.len(), 64);
        assert!(y0[..63].iter().all(|&b| b == 0));
        assert_eq!(y0[63], 3);

        // x = 3, n = 100, level = 2 => 3^4 mod 100 = 81
        let y2 = solve_rsa_puzzle(&[3], &[100], 2);
        assert_eq!(y2.len(), 64);
        assert!(y2[..63].iter().all(|&b| b == 0));
        assert_eq!(y2[63], 81);
    }

    #[test]
    fn test_solve_rsa_puzzle_zero_modulus_and_one_modulus() {
        // n = 0 should safely return 64 zero bytes without panicking
        let y_zero_n = solve_rsa_puzzle(&[0x12, 0x34], &[0, 0, 0], 10);
        assert_eq!(y_zero_n, vec![0u8; 64]);

        // Dummy challenge from TsCrypt.cs:440-442: x = 1, n = 1, level = 1 => y = 0
        let mut x = [0u8; 64];
        let mut n = [0u8; 64];
        x[63] = 1;
        n[63] = 1;
        let y_one = solve_rsa_puzzle(&x, &n, 1);
        assert_eq!(y_one, vec![0u8; 64]);
    }

    #[test]
    fn test_solve_rsa_puzzle_padding_and_truncation() {
        // Result requiring left-padding to 64 bytes
        let x = [0x05u8];
        let n = [0x01u8, 0x00u8]; // 256
        // 5^(2^1) mod 256 = 25
        let res = solve_rsa_puzzle(&x, &n, 1);
        assert_eq!(res.len(), 64);
        assert!(res[..63].iter().all(|&b| b == 0));
        assert_eq!(res[63], 25);

        // Modulus > 64 bytes producing a 65-byte residue should keep the last 64 bytes
        let mut big_x = vec![0xFFu8; 65];
        big_x[0] = 0x01;
        let mut big_n = vec![0xFFu8; 65];
        big_n[0] = 0x02;
        let res_trunc = solve_rsa_puzzle(&big_x, &big_n, 0);
        assert_eq!(res_trunc.len(), 64);
        assert_eq!(res_trunc, vec![0xFFu8; 64]);
    }

    #[test]
    fn test_solve_rsa_puzzle_512bit_realistic_vector() {
        // Realistic 512-bit (64-byte) x and odd modulus n with MSB set
        let mut x_be = [0u8; 64];
        let mut n_be = [0u8; 64];
        for (i, b) in x_be.iter_mut().enumerate() {
            *b = (i as u8).wrapping_mul(37).wrapping_add(11);
        }
        for (i, b) in n_be.iter_mut().enumerate() {
            *b = (i as u8).wrapping_mul(73).wrapping_add(0x81);
        }
        n_be[0] |= 0x80;
        n_be[63] |= 0x01; // odd modulus

        let level = 12u32;
        let actual = solve_rsa_puzzle(&x_be, &n_be, level);
        assert_eq!(actual.len(), 64);

        // Verify against direct BigUint::modpow with exponent 2^level
        let x = BigUint::from_bytes_be(&x_be);
        let n = BigUint::from_bytes_be(&n_be);
        let exp = BigUint::from(1u32) << level;
        let expected_big = x.modpow(&exp, &n);
        let expected_raw = expected_big.to_bytes_be();
        let mut expected = vec![0u8; 64];
        expected[64 - expected_raw.len()..].copy_from_slice(&expected_raw);

        assert_eq!(actual, expected);
    }
}
