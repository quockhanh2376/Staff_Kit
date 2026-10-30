//! Recovery Verification Gate and Fail-Closed Engine (D3.30 & P1.4 USB Mirror).
//!
//! Evaluates whether code-based recovery is permitted or must fail closed.
//!
//! Rules:
//! - If `usb_backup_enabled` is true:
//!   - USB destination must be connected.
//!   - Connected volume identity must match registered device ID.
//!   - Local ledger and USB ledger mirror must both be valid, with identical sequence & hash.
//!   - Missing USB, bad volume, or diverged ledger $\rightarrow$ `Disabled(reason)` (Fail-Closed).
//! - If `usb_backup_enabled` is false:
//!   - Local ledger must be valid and intact.
//!   - If local ledger is missing or corrupt $\rightarrow$ `Disabled(reason)`.
//!   - If valid $\rightarrow$ `Permitted(LocalOnly)`.

use super::ledger::RecoveryLedger;
use rusqlite::Connection;
use std::path::{Path, PathBuf};

pub const LEDGER_FILENAME: &str = "ledger.rev";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecoveryGateStatus {
    /// Code recovery is permitted with dual-copy verification.
    PermittedDualCopy {
        local_seq: u64,
        usb_seq: u64,
    },
    /// Code recovery is permitted on local ledger only (USB backup not configured).
    PermittedLocalOnly {
        local_seq: u64,
    },
    /// Code recovery is disabled (Fail-Closed). Password login remains 100% available.
    Disabled {
        reason: String,
        is_usb_offline: bool,
    },
}

pub struct VerificationGate;

impl VerificationGate {
    /// Evaluate recovery gate using the runtime database settings and paths.
    pub fn evaluate(
        conn: &Connection,
        local_ledger_path: &Path,
        master_key: &[u8],
    ) -> RecoveryGateStatus {
        // 1. Verify local ledger integrity first
        if !local_ledger_path.exists() {
            return RecoveryGateStatus::Disabled {
                reason: "Local recovery revocation ledger is missing.".to_string(),
                is_usb_offline: false,
            };
        }

        let local_ledger = match RecoveryLedger::open_and_verify(local_ledger_path, master_key) {
            Ok(ledger) => ledger,
            Err(err) => {
                return RecoveryGateStatus::Disabled {
                    reason: format!("Local recovery ledger verification failed: {err}"),
                    is_usb_offline: false,
                };
            }
        };

        // 2. Query P1.4 USB Backup settings
        let usb_enabled: bool = crate::db::get_setting_value(conn, crate::db::USB_BACKUP_ENABLED_SETTING_KEY)
            .unwrap_or(None)
            .map(|v| v.trim() == "1")
            .unwrap_or(false);

        if !usb_enabled {
            return RecoveryGateStatus::PermittedLocalOnly {
                local_seq: local_ledger.last_seq,
            };
        }

        // USB backup is enabled: strict dual-copy verification is mandatory
        let usb_dir_str = match crate::db::get_setting_value(conn, crate::db::USB_BACKUP_PATH_SETTING_KEY) {
            Ok(Some(s)) if !s.trim().is_empty() => s.trim().to_string(),
            _ => {
                return RecoveryGateStatus::Disabled {
                    reason: "USB backup directory path not configured.".to_string(),
                    is_usb_offline: true,
                };
            }
        };

        let registered_device_id = match crate::db::get_setting_value(conn, crate::db::USB_BACKUP_DEVICE_ID_SETTING_KEY) {
            Ok(Some(s)) if !s.trim().is_empty() => s.trim().to_string(),
            _ => {
                return RecoveryGateStatus::Disabled {
                    reason: "Registered USB device ID not configured.".to_string(),
                    is_usb_offline: true,
                };
            }
        };

        let usb_dir = PathBuf::from(&usb_dir_str);
        if !usb_dir.exists() {
            return RecoveryGateStatus::Disabled {
                reason: format!(
                    "Registered USB storage destination is disconnected or unavailable ({}). Connect registered USB media or sign in using your Super Admin password.",
                    usb_dir.display()
                ),
                is_usb_offline: true,
            };
        }

        // Check hardware volume identity
        let identity = match crate::db::storage_safety::get_volume_identity(&usb_dir) {
            Ok(id) => id,
            Err(err) => {
                return RecoveryGateStatus::Disabled {
                    reason: format!("Failed to read USB volume identity: {err}"),
                    is_usb_offline: true,
                };
            }
        };

        if identity.volume_serial_number != registered_device_id {
            return RecoveryGateStatus::Disabled {
                reason: format!(
                    "USB device mismatch: connected volume ({}) does not match registered USB device ({}).",
                    identity.volume_serial_number, registered_device_id
                ),
                is_usb_offline: true,
            };
        }

        // Check USB mirror ledger
        let usb_ledger_path = usb_dir.join(LEDGER_FILENAME);
        if !usb_ledger_path.exists() {
            return RecoveryGateStatus::Disabled {
                reason: "USB ledger mirror is missing on registered storage.".to_string(),
                is_usb_offline: false,
            };
        }

        let usb_ledger = match RecoveryLedger::open_and_verify(&usb_ledger_path, master_key) {
            Ok(ledger) => ledger,
            Err(err) => {
                return RecoveryGateStatus::Disabled {
                    reason: format!("USB ledger mirror is corrupt: {err}"),
                    is_usb_offline: false,
                };
            }
        };

        // Dual-copy consensus check
        if local_ledger.header.ledger_uuid != usb_ledger.header.ledger_uuid {
            return RecoveryGateStatus::Disabled {
                reason: "Ledger UUID mismatch between local and USB mirror.".to_string(),
                is_usb_offline: false,
            };
        }

        if local_ledger.last_seq != usb_ledger.last_seq {
            return RecoveryGateStatus::Disabled {
                reason: format!(
                    "Ledger sequence divergence: local seq ({}) != USB mirror seq ({}).",
                    local_ledger.last_seq, usb_ledger.last_seq
                ),
                is_usb_offline: false,
            };
        }

        if local_ledger.last_block_hash != usb_ledger.last_block_hash {
            return RecoveryGateStatus::Disabled {
                reason: "Ledger hash chain mismatch between local and USB mirror.".to_string(),
                is_usb_offline: false,
            };
        }

        RecoveryGateStatus::PermittedDualCopy {
            local_seq: local_ledger.last_seq,
            usb_seq: usb_ledger.last_seq,
        }
    }

    /// Sync local ledger to USB mirror if USB is configured and connected.
    pub fn sync_to_usb_mirror(
        conn: &Connection,
        local_ledger_path: &Path,
    ) -> Result<PathBuf, String> {
        let usb_enabled: bool = crate::db::get_setting_value(conn, crate::db::USB_BACKUP_ENABLED_SETTING_KEY)
            .unwrap_or(None)
            .map(|v| v.trim() == "1")
            .unwrap_or(false);

        if !usb_enabled {
            return Err("USB backup is not enabled".to_string());
        }

        let usb_dir_str = crate::db::get_setting_value(conn, crate::db::USB_BACKUP_PATH_SETTING_KEY)?
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty())
            .ok_or_else(|| "USB backup directory path not configured".to_string())?;

        let registered_device_id = crate::db::get_setting_value(conn, crate::db::USB_BACKUP_DEVICE_ID_SETTING_KEY)?
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty())
            .ok_or_else(|| "USB device ID not configured".to_string())?;

        let usb_dir = PathBuf::from(&usb_dir_str);
        if !usb_dir.exists() {
            return Err("USB backup destination disconnected".to_string());
        }

        let identity = crate::db::storage_safety::get_volume_identity(&usb_dir)?;
        if identity.volume_serial_number != registered_device_id {
            return Err("Connected USB volume does not match registered device".to_string());
        }

        let usb_ledger_path = usb_dir.join(LEDGER_FILENAME);
        let tmp_usb_ledger_path = usb_dir.join(format!("{LEDGER_FILENAME}.tmp"));

        // Copy atomically to tmp then rename on USB
        std::fs::copy(local_ledger_path, &tmp_usb_ledger_path)
            .map_err(|e| format!("failed to copy ledger to USB tmp: {e}"))?;

        // Durable sync
        let file = std::fs::File::open(&tmp_usb_ledger_path)
            .map_err(|e| format!("failed to open tmp USB ledger for sync: {e}"))?;
        file.sync_all()
            .map_err(|e| format!("failed to sync tmp USB ledger: {e}"))?;
        drop(file);

        std::fs::rename(&tmp_usb_ledger_path, &usb_ledger_path)
            .map_err(|e| format!("failed to commit USB ledger: {e}"))?;

        Ok(usb_ledger_path)
    }
}
