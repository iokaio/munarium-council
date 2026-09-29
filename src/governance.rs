// SPDX-License-Identifier: Apache-2.0
//! Advance a governance candidate through the permitted lifecycle.
//!
//! Candidate, tested, ratified, scheduled, activated and superseded are distinct; cancellation, expiry, veto and failed activation stay visible.
//!
//! Proposed local interface only. No implementation or wire format is provided.

/// Proposed boundary for: advance a governance candidate through the permitted lifecycle.
///
/// Implementations and concrete types await the component design and hub contracts.
/// This declaration does not enforce authentication, authorization, or durability.
pub trait GovernanceWorkflow {
    /// Input whose concrete shape and validation rules are still to be specified.
    type TransitionRequest;
    /// Output whose concrete shape and evidence requirements are still to be specified.
    type TransitionRecord;
    /// Failure reported without manufacturing a successful or authorized result.
    type Error;

    /// Advance a governance candidate through the permitted lifecycle.
    ///
    /// # Errors
    ///
    /// Implementations must report failed validation or unavailable required dependencies.
    fn transition(
        &mut self,
        input: &Self::TransitionRequest,
    ) -> Result<Self::TransitionRecord, Self::Error>;
}
