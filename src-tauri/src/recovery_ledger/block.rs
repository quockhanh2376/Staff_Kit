//! Recovery Revocation Ledger Block format and HMAC serialization.
//!
//! Magic Header: `SKREV1` (6 bytes)
//! Block binary layout:
//! - seq: u64 (big-endian, 8 bytes)
//! - prev_hash: [u8; 32] (32 bytes)
//! - timestamp: i64 (big-endian unix seconds, 8 bytes)
//! - account_id: i64 (big-endian, 8 bytes)
//! - event_type: u8 (1 byte: 1 = BATCH_ACTIVATED, 2 = CODE_BURNED, 3 = BATCH_REVOKED, 4 = EPOCH_RESET)
//! - payload_len: u16 (big-endian, 2 bytes)
//! - payload: bytes (variable, up to 1024 bytes)
//! - hmac: [u8; 32] (32 bytes HMAC-SHA256 over all preceding block fields)

use super::key_derivation::{hmac_sha256, sha256_digest};
use std::io::{self, Read};

pub const LEDGER_MAGIC: &[u8; 6] = b"SKREV1";
pub const HEADER_LEN: usize = 6 + 16 + 8; // magic(6) + uuid(16) + created_at(8) = 30 bytes

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum LedgerEventType {
    BatchActivated = 1,
    CodeBurned = 2,
    BatchRevoked = 3,
    EpochReset = 4,
}

impl LedgerEventType {
    pub fn from_u8(val: u8) -> Result<Self, String> {
        match val {
            1 => Ok(Self::BatchActivated),
            2 => Ok(Self::CodeBurned),
            3 => Ok(Self::BatchRevoked),
            4 => Ok(Self::EpochReset),
            other => Err(format!("unknown ledger event type: {other}")),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LedgerHeader {
    pub magic: [u8; 6],
    pub ledger_uuid: [u8; 16],
    pub created_at_unix: i64,
}

impl LedgerHeader {
    pub fn new(ledger_uuid: [u8; 16], created_at_unix: i64) -> Self {
        Self {
            magic: *LEDGER_MAGIC,
            ledger_uuid,
            created_at_unix,
        }
    }

    pub fn serialize(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(HEADER_LEN);
        buf.extend_from_slice(&self.magic);
        buf.extend_from_slice(&self.ledger_uuid);
        buf.extend_from_slice(&self.created_at_unix.to_be_bytes());
        buf
    }

    pub fn deserialize<R: Read>(reader: &mut R) -> io::Result<Self> {
        let mut magic = [0u8; 6];
        reader.read_exact(&mut magic)?;
        if &magic != LEDGER_MAGIC {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("invalid ledger magic: {:?}", magic),
            ));
        }

        let mut ledger_uuid = [0u8; 16];
        reader.read_exact(&mut ledger_uuid)?;

        let mut ts_buf = [0u8; 8];
        reader.read_exact(&mut ts_buf)?;
        let created_at_unix = i64::from_be_bytes(ts_buf);

        Ok(Self {
            magic,
            ledger_uuid,
            created_at_unix,
        })
    }

    /// Header digest used as the `prev_hash` for Block #1.
    pub fn digest(&self) -> [u8; 32] {
        sha256_digest(&self.serialize())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LedgerBlock {
    pub seq: u64,
    pub prev_hash: [u8; 32],
    pub timestamp_unix: i64,
    pub account_id: i64,
    pub event_type: LedgerEventType,
    pub payload: Vec<u8>,
    pub hmac: [u8; 32],
}

impl LedgerBlock {
    /// Construct and sign a new block with the given LedgerKey.
    pub fn new(
        seq: u64,
        prev_hash: [u8; 32],
        timestamp_unix: i64,
        account_id: i64,
        event_type: LedgerEventType,
        payload: Vec<u8>,
        ledger_key: &[u8; 32],
    ) -> Self {
        let mut block = Self {
            seq,
            prev_hash,
            timestamp_unix,
            account_id,
            event_type,
            payload,
            hmac: [0u8; 32],
        };
        let unsigned_bytes = block.serialize_unsigned();
        block.hmac = hmac_sha256(ledger_key, &unsigned_bytes);
        block
    }

    /// Serialize block fields excluding HMAC.
    pub fn serialize_unsigned(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(8 + 32 + 8 + 8 + 1 + 2 + self.payload.len());
        buf.extend_from_slice(&self.seq.to_be_bytes());
        buf.extend_from_slice(&self.prev_hash);
        buf.extend_from_slice(&self.timestamp_unix.to_be_bytes());
        buf.extend_from_slice(&self.account_id.to_be_bytes());
        buf.push(self.event_type as u8);
        buf.extend_from_slice(&(self.payload.len() as u16).to_be_bytes());
        buf.extend_from_slice(&self.payload);
        buf
    }

    /// Full serialized block including HMAC.
    pub fn serialize(&self) -> Vec<u8> {
        let mut buf = self.serialize_unsigned();
        buf.extend_from_slice(&self.hmac);
        buf
    }

    /// Compute SHA-256 digest of this block (used as prev_hash for the next block).
    pub fn block_hash(&self) -> [u8; 32] {
        sha256_digest(&self.serialize())
    }

    /// Verify HMAC signature against `ledger_key`.
    pub fn verify_signature(&self, ledger_key: &[u8; 32]) -> bool {
        let expected_hmac = hmac_sha256(ledger_key, &self.serialize_unsigned());
        // Constant-time compare
        let mut diff = 0u8;
        for (a, b) in self.hmac.iter().zip(expected_hmac.iter()) {
            diff |= a ^ b;
        }
        diff == 0
    }

    pub fn deserialize<R: Read>(reader: &mut R) -> io::Result<Self> {
        let mut seq_buf = [0u8; 8];
        reader.read_exact(&mut seq_buf)?;
        let seq = u64::from_be_bytes(seq_buf);

        let mut prev_hash = [0u8; 32];
        reader.read_exact(&mut prev_hash)?;

        let mut ts_buf = [0u8; 8];
        reader.read_exact(&mut ts_buf)?;
        let timestamp_unix = i64::from_be_bytes(ts_buf);

        let mut acc_buf = [0u8; 8];
        reader.read_exact(&mut acc_buf)?;
        let account_id = i64::from_be_bytes(acc_buf);

        let mut evt_buf = [0u8; 1];
        reader.read_exact(&mut evt_buf)?;
        let event_type = LedgerEventType::from_u8(evt_buf[0])
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

        let mut len_buf = [0u8; 2];
        reader.read_exact(&mut len_buf)?;
        let payload_len = u16::from_be_bytes(len_buf) as usize;

        let mut payload = vec![0u8; payload_len];
        reader.read_exact(&mut payload)?;

        let mut hmac = [0u8; 32];
        reader.read_exact(&mut hmac)?;

        Ok(Self {
            seq,
            prev_hash,
            timestamp_unix,
            account_id,
            event_type,
            payload,
            hmac,
        })
    }
}
