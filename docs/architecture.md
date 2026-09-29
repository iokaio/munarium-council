# Munarium Council implementation architecture

**Proposed design; scaffold only.** Based on section 10 of the
[platform plan, revision 4](https://github.com/iokaio/munarium-platform/blob/main/docs/platform-plan.md), with lifecycle and failure rules in
sections 17–19 and 22. See the hub's
[scaffold decision proposal](https://github.com/iokaio/munarium-platform/blob/main/docs/decisions/0001-scaffold-boundaries.md)
for the distinction between local interfaces and normative contracts.

## Responsibility and current boundary

Request-bound approval, distinct ratification, and recorded governance activation. Council belongs to the **authority plane**.
The crate declares interfaces only: no concrete implementations, serialization,
network listeners, persistence, service authentication or target operations exist.

The associated input, output and error types are intentionally unspecified.
These are proposed in-process seams for implementation work, not a released Rust API
or a second definition of the shared wire contract. A trait signature does not enforce
the trust assumptions below. Async runtime, transport and storage choices remain open.

## Module map

| Source | Proposed interface | Responsibility |
|---|---|---|
| [approval](../src/approval.rs) | `ApprovalService` | Verify approver, role, request binding, expiry, evidence, and quorum. A ticket status or chat message is presentation, not authority. |
| [governance](../src/governance.rs) | `GovernanceWorkflow` | Candidate, tested, ratified, scheduled, activated and superseded are distinct; cancellation, expiry, veto and failed activation stay visible. |
| [activation](../src/activation.rs) | `ActivationCoordinator` | Activation verifies expected prior state and digest through Registry or Server. Ratification alone must not mark an artifact effective. |

## Planned flow and state ownership

Capture exact approval request → authenticate distinct approver → record bound decision → revalidate freshness at execution. Governance candidates pass testing, ratification and scheduling before a separate Registry/Server activation acknowledgement can mark them effective.

Own approval decisions and governance workflow state with authoritative Server references. Registry and Server own their effective pointers. No target operations, target credentials, or direct writes that bypass those activation services belong in Council.

## Dependencies and failure behavior

| Dependency | Required input or service | Failure rule |
|---|---|---|
| Gate | Validated request digest, inputs, obligations and preconditions | Changed request or policy context requires fresh authority. |
| Warden / identity provider | Verified approver origin and permitted roles | Self-ratification and agent-derived ratifiers are refused. |
| Registry / Server | Activation with expected prior state | Stale state returns conflict; failed activation stays ineffective. |
| Server S1 / bootstrap | Explicit owner-provisioned non-agent transition path | Bootstrap identity, scope and retirement must be recorded; no hidden bypass. |

No dependency is linked into this scaffold. Supported contract versions are **none**.
Future adapters must consume a reviewed, versioned contract and identify its digest;
a floating hub branch is design context, never deployment authority.

## Threat assumptions

Treat agent code, supplied content and self-reported identity as untrusted.
Host administrators, release roots and required signing authorities remain explicit
trust assumptions of a qualified deployment. Process separation alone does not prove
independent administration.

| Threat | Required control to implement and test |
|---|---|
| Self-ratification through delegation | Verified proposer chain and separate permitted ratifier. |
| Approval substitution or stale button | Bind exact request/context and recheck validity; notifications confer no authority. |
| Activation race or emergency bypass | Compare-and-set transition, known-digest rollback, bounded human break-glass. |

The [validation specification](validation.md) connects these requirements to the hub
invariants. No test evidence is implied by this design.

## Decisions needed before implementation

Settle approval/activation envelopes, quorum and proposer-chain rules, bootstrap retirement, and the activation transaction protocol in the hub. The minimal authenticated CLI must work before a Console or ticketing integration is required.

A cross-component semantic change starts in a hub decision record. Keep publication,
activation and component implementation separate. Use expand, migrate, remove for
future breaking contract changes; never duplicate hashing, identity or grant rules.

## Deferred scope

Rich workflow design, many ticketing vendors, and any two-human review claim without two actual accountable people.

The [implementation plan](implementation-plan.md) sequences the first useful increment.
No deployment recipe, service port or live-provider configuration is supplied at this stage.
