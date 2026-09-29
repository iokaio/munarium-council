// SPDX-License-Identifier: Apache-2.0
//! Record an authenticated decision about an exact request.
//!
//! Verify approver, role, request binding, expiry, evidence, and quorum. A ticket status or chat message is presentation, not authority.
//!
//! Proposed local interface only. No implementation or wire format is provided.

/// Proposed boundary for: record an authenticated decision about an exact request.
///
/// Implementations and concrete types await the component design and hub contracts.
/// This declaration does not enforce authentication, authorization, or durability.
pub trait ApprovalService {
    /// Input whose concrete shape and validation rules are still to be specified.
    type ApprovalRequest;
    /// Output whose concrete shape and evidence requirements are still to be specified.
    type ApprovalRecord;
    /// Failure reported without manufacturing a successful or authorized result.
    type Error;

    /// Record an authenticated decision about an exact request.
    ///
    /// # Errors
    ///
    /// Implementations must report failed validation or unavailable required dependencies.
    fn decide(
        &mut self,
        input: &Self::ApprovalRequest,
    ) -> Result<Self::ApprovalRecord, Self::Error>;
}
