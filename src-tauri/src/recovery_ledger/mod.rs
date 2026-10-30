//! Module declarations and public API for Recovery Revocation Ledger.

pub mod block;
pub mod journal;
pub mod key_derivation;
pub mod ledger;
pub mod reconciler;
pub mod verification_gate;

#[cfg(test)]
pub mod tests;

// Re-exports
pub use block::{LedgerBlock, LedgerEventType, LedgerHeader};
pub use journal::{JournalState, RecoveryJournal, RecoveryJournalEntry};
pub use key_derivation::derive_ledger_key;
pub use ledger::RecoveryLedger;
pub use reconciler::RecoveryTransactionCoordinator;
pub use verification_gate::{RecoveryGateStatus, VerificationGate, LEDGER_FILENAME};
