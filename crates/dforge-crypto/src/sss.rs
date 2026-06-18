// Shamir's Secret Sharing — (k,n) threshold scheme
// Mathematics: polynomial f(x) over GF(2^8) (Galois Field)
//   f(x) = secret + a1*x + a2*x² + ... + a(k-1)*x^(k-1)
//   Shares: S_i = f(i) for i in 1..=n
//   Any k shares reconstruct secret via Lagrange interpolation
//   Any k-1 shares reveal ZERO information (information-theoretic security)
//
// We use GF(2^8) arithmetic — all ops in bytes, no big integers needed
// Time: O(k²) to reconstruct — with k=2 that's O(4) — essentially constant

use anyhow::Result;
use rand::Rng;
use serde::{Deserialize, Serialize};

// GF(2^8) with irreducible polynomial x^8 + x^4 + x^3 + x + 1 (AES polynomial)
const PRIMITIVE_POLY: u16 = 0x11b;

fn gf_mul(mut a: u8, mut b: u8) -> u8 {
    let mut result = 0u8;
    while b > 0 {
        if b & 1 != 0 {
            result ^= a;
        }
        let hi = a & 0x80;
        a <<= 1;
        if hi != 0 {
            a ^= (PRIMITIVE_POLY & 0xff) as u8;
        }
        b >>= 1;
    }
    result
}

fn gf_div(a: u8, b: u8) -> u8 {
    if b == 0 { panic!("division by zero in GF(2^8)"); }
    if a == 0 { return 0; }
    gf_mul(a, gf_pow(b, 254)) // Fermat's little theorem: b^(p-1) = 1 → b^(-1) = b^(p-2)
}

fn gf_pow(base: u8, exp: u8) -> u8 {
    let mut result = 1u8;
    let mut b = base;
    let mut e = exp;
    while e > 0 {
        if e & 1 != 0 { result = gf_mul(result, b); }
        b = gf_mul(b, b);
        e >>= 1;
    }
    result
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Share {
    pub index: u8,       // x-coordinate (1-indexed, never 0)
    pub data: Vec<u8>,   // y-coordinates (one per secret byte)
}

impl Share {
    pub fn to_hex(&self) -> String {
        format!("{:02x}{}", self.index, hex::encode(&self.data))
    }

    pub fn from_hex(s: &str) -> Result<Self> {
        if s.len() < 2 { anyhow::bail!("share too short"); }
        let index = u8::from_str_radix(&s[..2], 16)?;
        let data = hex::decode(&s[2..])?;
        Ok(Self { index, data })
    }
}

// Split secret into n shares, any k reconstruct
// secret: arbitrary bytes (our 44-byte AES key+nonce bundle)
pub fn split(secret: &[u8], k: u8, n: u8) -> Result<Vec<Share>> {
    if k < 2 { anyhow::bail!("k must be >= 2"); }
    if n < k { anyhow::bail!("n must be >= k"); }
    if k > 255 || n > 255 { anyhow::bail!("k and n must be <= 255"); }

    let mut rng = rand::thread_rng();
    let secret_len = secret.len();

    // For each secret byte, build a degree-(k-1) polynomial
    // f(0) = secret[i], coefficients a1..a(k-1) are random
    let mut shares: Vec<Share> = (1..=n).map(|i| Share { index: i, data: Vec::with_capacity(secret_len) }).collect();

    for &byte in secret {
        // Polynomial coefficients: [secret_byte, rand, rand, ...]
        let mut coeffs = Vec::with_capacity(k as usize);
        coeffs.push(byte);
        for _ in 1..k {
            coeffs.push(rng.gen());
        }

        // Evaluate polynomial at x=1,2,...,n using Horner's method
        // Horner: f(x) = c0 + x*(c1 + x*(c2 + ...)) — O(k) per point
        for share in shares.iter_mut() {
            let x = share.index;
            let mut val = *coeffs.last().unwrap();
            for &c in coeffs.iter().rev().skip(1) {
                val = gf_mul(val, x) ^ c;
            }
            share.data.push(val);
        }
    }

    Ok(shares)
}

// Reconstruct secret from any k shares using Lagrange interpolation
// L_i(0) = Π_{j≠i} (0 - x_j) / (x_i - x_j)  in GF(2^8)
// secret[b] = Σ_i share_i.data[b] * L_i(0)
pub fn reconstruct(shares: &[Share]) -> Result<Vec<u8>> {
    if shares.is_empty() { anyhow::bail!("no shares provided"); }
    let secret_len = shares[0].data.len();

    // Verify all shares have same length
    for s in shares.iter() {
        if s.data.len() != secret_len {
            anyhow::bail!("share length mismatch");
        }
    }

    let k = shares.len();
    let xs: Vec<u8> = shares.iter().map(|s| s.index).collect();
    let mut secret = vec![0u8; secret_len];

    // Lagrange interpolation at x=0
    // O(k²) total — with k=2: exactly 4 GF multiplications
    for b in 0..secret_len {
        let ys: Vec<u8> = shares.iter().map(|s| s.data[b]).collect();
        let mut val = 0u8;

        for i in 0..k {
            let mut num = 1u8;
            let mut den = 1u8;
            for j in 0..k {
                if i == j { continue; }
                // In GF(2^8): subtraction = XOR = addition
                num = gf_mul(num, xs[j]); // numerator: Π x_j
                den = gf_mul(den, xs[i] ^ xs[j]); // denominator: Π (x_i - x_j)
            }
            val ^= gf_mul(ys[i], gf_div(num, den));
        }
        secret[b] = val;
    }

    Ok(secret)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_reconstruct_2_of_3() {
        let secret = b"this is a 44-byte AES key+nonce!xxxxxxxxxx";
        let shares = split(secret, 2, 3).unwrap();
        assert_eq!(shares.len(), 3);

        // Any 2 shares reconstruct
        let rec = reconstruct(&shares[..2]).unwrap();
        assert_eq!(rec, secret);

        let rec = reconstruct(&[shares[0].clone(), shares[2].clone()]).unwrap();
        assert_eq!(rec, secret);

        let rec = reconstruct(&[shares[1].clone(), shares[2].clone()]).unwrap();
        assert_eq!(rec, secret);
    }

    #[test]
    fn single_share_reveals_nothing() {
        let secret = vec![42u8; 44];
        let shares = split(&secret, 2, 3).unwrap();
        // One share cannot reconstruct — result would be wrong
        let wrong = reconstruct(&shares[..1]).unwrap();
        assert_ne!(wrong, secret);
    }

    #[test]
    fn hex_roundtrip() {
        let secret = vec![0xABu8; 44];
        let shares = split(&secret, 2, 3).unwrap();
        let hex = shares[0].to_hex();
        let parsed = Share::from_hex(&hex).unwrap();
        assert_eq!(parsed.index, shares[0].index);
        assert_eq!(parsed.data, shares[0].data);
    }
}
