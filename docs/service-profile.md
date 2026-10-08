# Experimental Stage 2 Council service

The [approval store](../src/store.rs), [coordinator](../src/coordinator.rs) and
[mTLS service](../src/main.rs) implement the first Council runtime packet under
hub ADR 0014. The [exported candidate](../contracts/stage2-v1/README.md) retains
bundle `8aca66588c87107a0c7a7720c68f921dfd2bdcaecf6433728afa4b1c51420aa6`.
No contract release, human acceptance or qualified target path is claimed.

## Interfaces and current authority

Run `munarium-council ABSOLUTE_CONFIG_JSON`. Configuration supplies the existing
Warden-exported mTLS settings, an absolute SQLite `database`, HTTPS
`server_endpoint`, `gate_endpoint`, `warden_endpoint`, optional
`registry_endpoint`, `deployment`, `service`, `server_service`, `provider_id` and
`provider_token_file`. Only enrolled leaf fingerprints are admitted. Identity
material is operator provisioned and must never enter the public tree.

Server's current governing artifact contains `stage2:<service>` with qualified
`scope`, `humans` mapping enrolled peers to `principal` and
`eligibility_revision`, `readers`, `stream`, `generation` and optional `proposers`.
The human path is separate operator enrollment, not a workload assertion that
claims human kind. The reference scope is account separation, not evidence of two
independent human operators. Missing authority or dependency access fails closed.

Both endpoints accept `{tenant, action}` with closed operation-specific inputs:

| Endpoint | Operations |
|---|---|
| `/v1/approvals` | `approve` (id, operation_id, attempt_id), `lookup` (id), `withdraw` (id, withdrawal_id), `flush` |
| `/v1/transitions` | `propose` (canonical transition string), `ratify`, `lookup`, `advance` (transition_id) |

Approval uses Gate's authenticated source lookup; the caller supplies no trusted
request, context, decision or human identity. Exact retries return the original
issuance and expiry. `currently_usable` is distinct from immutable issuance and
changes with current eligibility, context, status and time. Issuance has a
two-second uncertainty margin; callers must not use it before that interval.
No Council response is a target-send instruction.

Withdrawal first commits pending status. It becomes effective only after the
matching authenticated Gate cancellation receipt; a too-late receipt is retained
separately. Losing the reply never means successful cancellation.

Activation retains proposal, distinct-human ratification, Gate pause and each
participant's exact apply receipt. Participant calls do not hold Council's local
database lock. `advance` retries missing steps; incomplete receipts cannot produce
a resume command. A failed resume response leaves completion pending. The full
barrier additionally requires the Gate, Registry, Server and Warden adapters;
local coordinator tests alone do not establish that integration.

## Persistence and delivery

SQLite uses WAL, FULL synchronous durability and immediate write transactions.
Approval issuance, immutable lifecycle history and the approval event outbox commit
together. `flush` obtains a current Warden assertion, archives the approval and
delivers the original event to Server. Only an exact authenticated acknowledgement
marks delivery complete. Source sequence and predecessor survive restart.

Withdrawal and activation progress are retained in local immutable history.
Their complete platform audit/export coverage remains an integration obligation;
do not treat locally retained progress as a Server acknowledgement. Operator
restore is not qualified by reopening a SQLite file. Do not reuse a restored
Council database as proof that execution authority is safe.

## CLI and validation

The [CLI](../scripts/council_client.py) accepts `--endpoint`, `--ca`,
`--certificate`, `--key` and `--request` file arguments. Use the appropriate
enrolled human or service certificate; it refuses redirects and bounds responses.
There are no default keys or production endpoints.

```console
cargo fetch --locked
cargo fmt --all --check
cargo build --offline --locked
cargo clippy --offline --locked --all-targets -- -D warnings
cargo test --offline --locked
cargo doc --offline --locked --no-deps
python scripts/test_service.py -v
```

[Approval tests](../tests/approval.rs) cover independent candidate bytes/digests,
concurrent issuance, immutable retries, restart/outbox survival, changed bindings,
current eligibility, expiry, tenant refusal and withdrawal receipt binding.
[Activation tests](../tests/activation.rs) restart after every receipt boundary and
reject incomplete, changed or substituted receipts and self-ratification.
[Export tests](../tests/source_exports.rs) check owner-provided source/contract locks.
The service test starts the real binary and TLS endpoints with synthetic trusted
dependency servers. It checks transport enrollment and restart behavior; it is
not the full platform composition or an external identity-provider qualification.

The fixture generates disposable test keys and databases, stops its owned process
and servers and removes the temporary directory. It never creates target effects.
REF-02-19 and H03 composition results must be recorded separately in Harness.

## Approval custody integration

Approval lookup now includes `audit.event` and `audit.acknowledgement`, fetched from
Council's retained outbox after the reader is authenticated. A null acknowledgement
means recording remains pending; it never grants execution authority. Current status,
human eligibility and immutable expiry remain separate from that historical evidence.
The recorder requests Server's implemented `read`/`propose` scopes, using Council's
own workload identity. Gate's prepared release service supplies source records and
returns exact cancellation or too-late receipts for withdrawal. Harness exercises
this path with actual Council, Gate, Warden and Server processes.
