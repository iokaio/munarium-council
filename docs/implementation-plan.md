# Munarium Council build plan

**Proposed work; no functional milestone is complete.** The design baseline is the
[public platform plan, revision 4](https://github.com/iokaio/munarium-platform/blob/main/docs/platform-plan.md), section 10, and its
stage sequence in section 25. Council's initial delivery belongs to **Stage 1 interfaces; 2 action path**.
Calendar windows are planning targets; acceptance evidence controls advancement.

## Preparation present in this checkout

- A non-publishable, dependency-free Cargo library with documented interface modules.
- An [architecture map](architecture.md) naming ownership, trust assumptions and failures.
- An [acceptance specification](validation.md) and automatic Rust build checks.
- Existing contribution, security, support and repository-hygiene processes.

These artifacts prepare implementation; they do not complete Stage 0 foundation qualification
or advance this repository beyond the hub's **repository created** catalog state.

## First work packet: COUNCIL-01: request-bound approval through a minimal authenticated interface

**Prerequisites:** accepted hub decisions and the specific contracts named in
[Architecture](architecture.md); record the exact revisions used. All fixtures must be synthetic
or authorized public inputs. The hub [contract backlog](https://github.com/iokaio/munarium-platform/blob/main/docs/architecture/contract-backlog.md)
tracks unresolved cross-component definitions.

**Work:** After approval binding and principal contracts are accepted, implement a small approval workflow for a synthetic release action. Exercise a proposer and a distinct approver, visible evidence and expiry. Keep target dispatch and governance activation disconnected until their separate protocols exist.

**Permitted scope:** the relevant modules under `src/`, component-local tests/fixtures,
and their documentation. Add dependencies, runtime wiring, or migrations only when the packet
requires them and its owner has reviewed the design. Do not copy sibling implementations.

**Acceptance:** Changing artifact, recipient, amount, target, epoch, precondition or expiry invalidates authority. Proposer-chain and agent-derived ratification are refused. A ticket marked approved cannot authorize anything without the verified callback.

**Handoff:** retain commands, exit codes, fixture/contract revisions, limitations and the
diff for review. A test specification is not a passed test. Publishing, deployment, live
provider calls, signing changes and policy activation are separate operations.

## Subsequent packets

| Packet | Implementation scope | Exit condition |
|---|---|---|
| COUNCIL-02 | Implement candidate/tested/ratified/scheduled lifecycle and failure outcomes. | Veto, cancellation, expiry, and failed activation remain distinct and recorded. |
| COUNCIL-03 | Coordinate compare-and-set activation with Registry and Server. | Concurrent or stale activation cannot overwrite effective state; record bootstrap retirement. |
| COUNCIL-04 | Add bounded rollback and shadow impact reports. | Only a known prior digest can be restored under pre-authorized scope; report exclusions and reconstructed inputs. |

Each packet gets a concrete component issue and links to the coordinating hub issue when
execution begins. The identifiers above are local planning references, not claims that remote
issues or approvals already exist. Work advances one coherent capability slice at a time.

## Integration and operational readiness

Before any runtime capability is advertised, document its supported contracts, immutable source
revision, accepted dependency versions and deployment boundary. Demonstrate relevant failure
paths from [Validation](validation.md), then add the component runbook: required identities,
health and dependency states, migration order, backup/restore, key rotation where applicable,
and unresolved-work investigation.

A component result alone is not platform qualification. The hub's
[delivery sequence](https://github.com/iokaio/munarium-platform/blob/main/docs/build-plan.md) requires composition evidence, including the
Server/Matrix foundation and the authority path required by the selected consequence class.
Rich workflow design, many ticketing vendors, and any two-human review claim without two actual accountable people.

## Completion criteria for the first functional increment

- The documented local recipe works from a clean clone using bounded disposable inputs.
- The acceptance cases are executable, retain their intended oracle, and include refusal paths.
- Unsupported operations remain explicit; logs and reports expose no credentials or private data.
- The README links the actual evidence before any capability or release label changes.
