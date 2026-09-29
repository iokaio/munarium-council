// SPDX-License-Identifier: Apache-2.0
//! Request an independently authorized activation.
//!
//! Activation verifies expected prior state and digest through Registry or Server. Ratification alone must not mark an artifact effective.
//!
//! Proposed local interface only. No implementation or wire format is provided.

/// Proposed boundary for: request an independently authorized activation.
///
/// Implementations and concrete types await the component design and hub contracts.
/// This declaration does not enforce authentication, authorization, or durability.
pub trait ActivationCoordinator {
    /// Input whose concrete shape and validation rules are still to be specified.
    type RatifiedTransition;
    /// Output whose concrete shape and evidence requirements are still to be specified.
    type ActivationOutcome;
    /// Failure reported without manufacturing a successful or authorized result.
    type Error;

    /// Request an independently authorized activation.
    ///
    /// # Errors
    ///
    /// Implementations must report failed validation or unavailable required dependencies.
    fn activate(
        &mut self,
        input: &Self::RatifiedTransition,
    ) -> Result<Self::ActivationOutcome, Self::Error>;
}
