// SPDX-License-Identifier: Apache-2.0
//! Munarium Council: experimental request-bound approvals and activation coordination.
//!
//! Request-bound approval, distinct ratification, and recorded governance activation.
//!
//! Original interface traits remain available. The store and coordinator modules
//! add durable experimental implementations; the binary supplies enrolled mTLS.
//! Full participant integration and production qualification remain pending.
//! See `docs/architecture.md` and `docs/implementation-plan.md` in this repository.
//!
//! The interfaces are provisional and may change before the first implementation.
//! Concrete cross-component types must follow an accepted hub decision and contract.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

pub mod activation;
pub mod approval;
pub mod governance;

pub mod coordinator;
pub mod store;
pub mod wire;
