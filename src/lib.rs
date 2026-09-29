// SPDX-License-Identifier: Apache-2.0
//! Munarium Council: implementation scaffolding only.
//!
//! Request-bound approval, distinct ratification, and recorded governance activation.
//!
//! These modules declare proposed in-process interfaces with associated types.
//! There are no implementations, wire contracts, listeners, storage backends,
//! credentials, or runtime capabilities. No production path is qualified.
//! See `docs/architecture.md` and `docs/implementation-plan.md` in this repository.
//!
//! The interfaces are provisional and may change before the first implementation.
//! Concrete cross-component types must follow an accepted hub decision and contract.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

pub mod activation;
pub mod approval;
pub mod governance;
