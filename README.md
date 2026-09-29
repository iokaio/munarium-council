# Munarium Council

**Approvals, policy lifecycle, ratified governance transitions.** Council is the authority-plane
component of the Munarium Governance Platform that owns the lifecycle of approvals and activated
governance. It authorizes a precise request and records who or what supplied the authority; it
moves a governance change from candidate to activated through distinct, verified authority; and it
does not execute target operations or hold their credentials. Its central rule is the platform's:
**an agent's ordinary operating identity must never activate the rules that govern it.**

> **Status: Planned — Rust scaffold present.** This checkout contains a dependency-free,
> non-publishable [Cargo library](Cargo.toml) and documented interfaces under [src/](src/lib.rs).
> The interfaces have no implementations: no runtime service, client transport, database,
> provider integration or contract implementation is available. No production path is qualified.
> Build checks validate source structure, not governance capabilities. The
> [capability table](#capability-status) remains the authoritative functional status.

Council is one of nine components built around the existing Munarium foundation, Munarium Server
and Munarium Matrix. Their shared architecture, normative contracts, decision records, roadmap and
composition evidence live in the public hub,
[iokaio/munarium-platform](https://github.com/iokaio/munarium-platform). This repository will hold
Council's implementation, its unit and component tests, the migrations it owns, operational
diagnostics, package definitions, a local development recipe and release evidence. It is open
source from its first public commit, under the Apache License 2.0, with no proprietary edition:
advanced Council workflows are deferred roadmap work, not a closed-edition feature.

## Start building

Read the [development index](docs/README.md), then the [architecture](docs/architecture.md),
[implementation plan](docs/implementation-plan.md) and [validation guide](docs/validation.md).
They map the public platform plan to source modules, dependencies, a first bounded work item
and acceptance cases. Runtime capabilities remain planned; supported contract versions are **none**.

## What Council is for

The platform separates four powers: **read**, **governed write**, **act** and **govern**. Council
is the authority behind *govern*, and it supplies the distinct approval that a high-consequence
*act* requires. The platform distinguishes **proposing** a governance change from **activating**
governance: an agent can submit an inert proposal through a narrow intake surface, and that does not
make the agent a writer to the active authority state. Registry's effective manifests, Council's
ratification state, trust roots and Server's active governance transitions all remain outside the
agent's ordinary authority.

Separating credentials is necessary but not sufficient. Two credentials controlled by one person do
not create two independent human reviewers, and Council's records say who actually supplied each
authority.

## The design, as planned

### Action approval

An approval request includes the canonical request hash, target and environment, policy and
manifest digests, evidence references, required obligations, expiry and principal chain.
**Approval is specific enough that a changed recipient, amount, artifact or target invalidates it.**

An ITSM ticket may carry an approval task and its evidence, but a ticket's status string is not
sufficient authority. Council verifies the approver, role, decision, request binding, freshness and
any required quorum. Notifications in Teams, Slack, email or a portal are presentation surfaces;
**the authenticated callback to Council is the authoritative action.**

The first release provides a minimal authenticated approval interface and a command-line workflow.
A rich Console and multiple ticketing vendors are not prerequisites for demonstrating the authority
boundary. Human approvers see the evidence packet beside the proposed effect, including unresolved
provenance, expected side effects and target-state preconditions.

### Governance changes

Governance changes follow **candidate, tested, ratified, scheduled, activated and superseded**
states. The record links the old and new artifact digests, proposer, approver, expected current
activation, affected deployment and reason. Cancellation, expiry, veto and failed activation are
first-class outcomes: a ratified artifact whose activation failed does not appear effective because
the approval step succeeded. Registry and Server activation use compare-and-set or equivalent
version checks so competing updates cannot both win.

**Shadow evaluation** replays a declared set of historical proposals under the candidate and reports
changed decisions, what was excluded, whether source evidence was available and which inputs were
reconstructed. It is an impact report, not proof that all future behavior is safe. Adversarial and
boundary tests are retained independently of recent traffic.

A principal cannot ratify a governance proposal that policy identifies as its own, and an
agent-derived identity cannot become the ratifier by delegating through another service. A
deterministic pipeline can supply approval only for change classes whose policy permits it, and only
if the pipeline's own active controls are outside the proposing agent's write authority.

### The lifecycle of a rule and the policy toolchain

The policy-change model is **propose, package, shadow, ratify, time-lock, activate, observe**, with
each state visible and versioned. A candidate carries a human-readable rationale, affected identities
and tools, schema and compatibility information, regression results and the expected impact on
denials and approvals; the machine-readable diff and test evidence remain necessary whatever a
generated explanation says.

Promotion verifies signer identity, expected current activation, target environment, approval
requirements and the candidate digest. A time-lock allows review for selected change classes. **A
policy that weakens enforcement is itself a governance change**: widening a tool, changing an
approver role, disabling required evidence, or moving an environment from enforce to observe.

Emergency rollback restores a specific previously approved digest within a bounded, pre-authorized
scope, with immediate recording and later review. It never lets an agent label an arbitrary new
bundle a rollback. Break-glass is a pre-authorized human procedure with narrow scope, a short
validity window, immediate recording and post-event review; it does not authorize an agent to
suspend its own governance.

**One policy language first.** The policy-engine question stays open until Gate's implementation
spike is complete, and the first release avoids two primary evaluators. Federated policy initially
favors separately evaluated mandatory parent constraints and local restrictions with deny-overrides
behavior, which is easier to reason about than a policy-merging compiler claiming to prove every
child policy stricter. The toolchain provides linting, schema checks, signed fixtures, policy replay
and a concise impact report: a small workflow executed from source, not a policy studio that becomes
a second ungoverned control surface.

### Bootstrap without circular dependencies

Server's governance transitions require a Council attestation, and Council depends on the
foundation. The platform breaks that cycle with an explicit **bootstrap contract**: before Council
exists, Server's new governance boundary accepts only a narrowly defined attestation from an
owner-provisioned, non-agent authority key, binding the intended transition and expected prior
state. Its identity, permitted transition classes, expiry or retirement condition and audit record
are part of the installation. Once Council is qualified, a recorded trust transition retires or
restricts the bootstrap key. The agent never receives either authority.

This allows the authority-role split (the hub's S1 change to Server) to be delivered before the
entire platform. It is a decision about how to build, not a claim that Server 1.3.0 already
supports the contract.

### Degraded operation

Council unavailable means **no new approval or governance activation**. Previously authorized work
proceeds only if its obligations, policy validity, expiry and other dependencies permit it. The
dependency contract is published rather than alternating between incompatible outage rules.

## First public increment

**A request-bound approval and an immutable activation**, through a minimal authenticated interface
and command-line workflow, as part of the platform's first complete governed action: Gate's durable
journal, Warden's first real identity and broker path, and Council's minimum approval and activation
workflow become one vertical slice with one narrow connector, one reference identity system and one
deployment profile.

Target window: Stage 2 (months 4–6); interfaces and a bounded prototype during Stage 1.

## Capability status

The labels are evidence labels, not editions: **Planned**, **Experimental**, **Conformance-tested**,
**Reference-qualified**, **Independently reviewed**. In the hub's component catalog this
repository is at **repository created**.

| Capability | Status | Evidence |
|---|---|---|
| Request-bound approval: hash, target, environment, digests, evidence, obligations, expiry, principal chain | Planned | none |
| Authenticated approval interface and command-line workflow | Planned | none |
| Approver, role, decision, binding, freshness and quorum verification on the callback | Planned | none |
| Governance state machine: candidate, tested, ratified, scheduled, activated, superseded; cancellation, expiry, veto, failed activation | Planned | none |
| Compare-and-set activation into Registry and Server | Planned | none |
| Bootstrap attestation contract and its recorded retirement | Planned, with Server's S1 | none |
| Shadow evaluation as an impact report | Planned | none |
| Policy toolchain: lint, schema checks, signed fixtures, replay, impact report | Planned | none |
| Time-lock for selected change classes; bounded pre-authorized rollback; break-glass procedure | Planned | none |
| One ITSM or approval integration with exact binding and authenticated callback | Planned, later (P1) | none |
| Federated policy: mandatory parent constraints, deny-overrides, tested prototype | Planned, later (stage 5) | none |
| Multiple ticketing vendors; rich approval UX (Console) | Deferred | none |

Supported contract versions: **none**. Supported approval integrations: **none**. Operations
available today: **none**.

## Acceptance evidence for the first release

| Test | Required outcome |
|---|---|
| A permitted effect requiring approval | Bound approval recorded; Gate proceeds; the chain links proposal, decision, approval, claim, grant, receipt |
| Substituted request after approval | Approval invalid; a new review is required |
| Stale approval, expired window or changed target precondition | Refused before dispatch |
| Attempted self-ratification | The active governance state is unchanged |
| Agent-derived identity presented as ratifier through delegation | Refused |
| Ticket status without an authenticated callback | Not authority; nothing activates |
| Concurrent activations | One wins by compare-and-set; the other sees a conflict |
| Failed activation after ratification | Not shown as effective |
| Bootstrap attestation outside its permitted classes or after retirement | Refused, with the audit record |
| Rollback naming a digest never previously approved | Refused |
| Council unavailable | No new approval or activation; previously authorized work follows the published dependency contract |

A blank evidence field means unverified, not passed. Where a risk class requires independent human
review, the release obtains it or stays outside the corresponding production claim.

## Invariants

| ID | Required property | Owner and first gate |
|---|---|---|
| INV-01 | Ordinary governed-write authority cannot activate a governance profile | Server and Council; stage 0 |
| INV-09 | Approval is invalid after relevant content, policy validity or target preconditions change | Council and Gate; stage 2 |
| INV-13 | A delegated principal cannot widen its originating scope or become its own ratifier | Warden and Council; stage 2 |
| INV-16 | Console cannot perform a privileged operation unavailable through governed APIs | Console, Council, Warden; stage 3 |
| INV-20 | A local overlay cannot waive a mandatory parent prohibition | Council, Registry, Gate; stage 5 |
| INV-22 | A release advertises only the profiles and capabilities supported by its evidence | every component; every stage |

INV-01 is the first substantive product change on the platform roadmap and is delivered through
Server's S1 and the bootstrap contract; a demonstrated refusal of an unauthorized governance
transition is stage 0 exit evidence, before Council itself exists.

## Contracts, dependencies and neighbors

- **Contracts.** The hub's contracts directory is normative for the approval request, the
  governance-transition record, the activation event and the bootstrap attestation. Council
  implements them. Supported contract versions: none yet.
- **Foundation.** Munarium Server 1.3.0 supplies the ledger the records land in; the hub's S1
  (separate governance authority from ordinary read-write and management roles, with a transition
  authorization contract and a non-agent bootstrap attestation path) is the Server change Council
  depends on and that precedes it.
- **Gate** asks for approval where the consequence class requires it and rechecks approval
  validity before dispatch; **Registry** is where activated manifests and policy bundles take
  effect; **Warden** never lets an agent-derived credential become a ratifier; **Console** calls
  Council's API for every approval and has no other path; **Matrix** assembles evidence packets for
  Council's approvers; **Assure** links approvals and activations into evidence packages.
- **External dependencies.** None chosen. One ITSM or approval integration follows at priority P1
  with exact request binding, authenticated approval, expiry and target-state checks.

## Not in scope

- Executing target operations or holding target credentials.
- Treating a ticket status, a chat reaction or a portal click as authority.
- Independent two-human review inside a one-person organization.
- Two primary policy evaluators, or a policy-merging compiler, in the first release.
- Editing an active bundle in place, or a rollback that is not a previously approved digest.
- A general workflow designer or a proprietary approval tier.

## Roadmap position

| Stage | Council's part |
|---|---|
| 0 · month 1 | This repository; the approval, transition and bootstrap contracts drafted in the hub; Server's S1 with the bootstrap attestation path; a demonstrated refusal of an unauthorized governance transition |
| 1 · months 2–3 | Interfaces and a bounded prototype; production activation disabled |
| 2 · months 4–6 | Minimum approval and activation workflow with a command-line interface: the first complete governed action, including attempted self-ratification and a substituted request |
| 3 · months 7–9 | Approver view in Console once the binding and authentication contracts are ready; one ITSM or approval integration |
| 4 · months 10–12 | Policy toolchain in the reference composition; approvals and activations in evidence packs |
| 5 · months 13+ | Advanced workflows, federation constraints, further integrations, demand-led and still open source |

Required distinct authority is never removed to preserve a date.

## Repository layout

| Path | What exists |
|---|---|
| [Cargo.toml](Cargo.toml), [Cargo.lock](Cargo.lock) | Independent library, version 0.1.0-dev, publishing disabled, no external crate dependencies |
| [src/lib.rs](src/lib.rs) | Documented proposed module interfaces; no runtime implementations |
| [docs/](docs/README.md) | Architecture, implementation sequence and acceptance specifications |
| [CONTRIBUTING.md](CONTRIBUTING.md), [AGENTS.md](AGENTS.md), [CLAUDE.md](CLAUDE.md) | Contribution process and aligned development guidance |
| [.github/workflows/](.github/workflows/) | Automatic Rust, repository-hygiene and DCO checks |
| [scripts/](scripts/), [check_license.py](check_license.py) | Existing documentation, private-material and license checks |
| [LICENSE](LICENSE), [NOTICE](NOTICE), [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md) | Licensing and dependency notices |

Subsystem modules: [approval](src/approval.rs), [governance](src/governance.rs), [activation](src/activation.rs).
Tests, fixtures, migrations, binaries and deployment assets arrive with the implementation that
uses them. The scaffold defines no shared wire types and depends on no sibling checkout.

## Development

Use Rust 1.98.1 with rustfmt, Clippy and the platform's native linker. From this repository root:

```console
cargo fmt --all --check
cargo build --offline --locked
cargo clippy --offline --locked --all-targets -- -D warnings
cargo test --offline --locked
cargo doc --offline --locked --no-deps
```

The crate currently has **zero runtime or conformance tests**. A successful test command checks
the scaffold only. The [validation guide](docs/validation.md) gives the required behavioral
test specifications and explains how to retain evidence when they are implemented.

Also run the existing hygiene gates:

```console
py check_license.py
py scripts/private_material_scan.py
py scripts/docs_linkcheck.py
gitleaks dir . --config .gitleaks.toml --no-banner --redact --exit-code 1
git diff --check
```

Use `python` or `python3` where `py` is unavailable. The new
[Rust workflow](.github/workflows/rust.yml) runs on main pushes and pull requests alongside
the existing [repository hygiene](.github/workflows/repo-hygiene.yml) and
[DCO](.github/workflows/dco.yml) workflows. They provide build and repository checks, not a
qualified runtime. No package is published or service deployed by these workflows.
Local checks do not imply hosted CI success. See [CONTRIBUTING.md](CONTRIBUTING.md).

## The platform

| Repository | Plane | Role |
|---|---|---|
| [iokaio/munarium-platform](https://github.com/iokaio/munarium-platform) | hub | Architecture, normative contracts, decision records, roadmap and composition evidence for the whole platform |
| [iokaio/munarium](https://github.com/iokaio/munarium) | foundation (mediation) | Munarium Server: governed memory, the append-only ledger, and the Server client libraries |
| [iokaio/munarium-matrix](https://github.com/iokaio/munarium-matrix) | foundation (mediation) | Munarium Matrix: governed, read-only structured evidence from enterprise data sources |
| [iokaio/munarium-registry](https://github.com/iokaio/munarium-registry) | authority | Inventory of agents, tools, manifests, and policy bundles |
| [iokaio/munarium-harness](https://github.com/iokaio/munarium-harness) | agent | SDKs that make the governed path easy for honest agents |
| [iokaio/munarium-warden](https://github.com/iokaio/munarium-warden) | authority | Workload identity, delegation, just-in-time credentials, kill switches |
| [iokaio/munarium-gate](https://github.com/iokaio/munarium-gate) | mediation | Policy decision and enforcement point for every tool call |
| [iokaio/munarium-gateway](https://github.com/iokaio/munarium-gateway) | mediation | Model-call mediation: routing, BYOK, budgets, screening |
| [iokaio/munarium-council](https://github.com/iokaio/munarium-council) | authority | Approvals, policy lifecycle, ratified governance transitions |
| [iokaio/munarium-sentinel](https://github.com/iokaio/munarium-sentinel) | assurance | Telemetry, anomaly detection, circuit breakers, incident replay |
| [iokaio/munarium-assure](https://github.com/iokaio/munarium-assure) | assurance | Control-framework mapping and evidence packs |
| [iokaio/munarium-console](https://github.com/iokaio/munarium-console) | assurance | One interface for approvers, operators, and auditors |
| [iokaio/munarium-clients-publish](https://github.com/iokaio/munarium-clients-publish) | tooling | The one place Munarium client packages are built for release and published from |
| [iokaio/munarium-demo](https://github.com/iokaio/munarium-demo) | examples | Munarium Demo: working applications and bundled datasets for evaluating the foundation |

The development tool VCP ([iokaio/vcp](https://github.com/iokaio/vcp)) is separate: not one of the
nine components and not a runtime dependency for adopters. Ioka's private repositories hold
planning material awaiting publication review and the proprietary Matrix analytics adapters;
nothing from them is copied into a public repository without that review.

## Licensing

Apache-2.0 ([LICENSE](LICENSE), [NOTICE](NOTICE)). The names are not part of that grant:
[TRADEMARK.md](TRADEMARK.md) says what you may do without asking, which is most things. There is
no proprietary edition of this component and none is planned; a capability that arrives later is
deferred roadmap work, not a commercial restriction.

## Contributing, support, security

Signed-off pull requests, no CLA ([CONTRIBUTING.md](CONTRIBUTING.md)). Questions go to Discussions,
defects and design findings to Issues, and suspected vulnerabilities to the private channel
[SECURITY.md](SECURITY.md) names, never a public issue. What is and is not supported:
[SUPPORT.md](SUPPORT.md). Conduct: [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md). Release history,
such as it is: [CHANGELOG.md](CHANGELOG.md).
