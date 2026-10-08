# Munarium Council validation

## Build the scaffold locally

The first runtime packet is now implemented; use the commands and coverage map in
the [service profile](service-profile.md). Fetch the pinned dependencies with
`cargo fetch --locked` before offline checks. The historical scaffold description
below records the original baseline and does not describe the new runtime tests.

Use Rust **1.98.1** with Cargo, rustfmt and Clippy, plus the platform's native linker.
The manifest requires Rust 1.98; older toolchains are not qualified by this scaffold.
CI installs 1.98.1 explicitly. No provider account, database, model key, container, sibling
checkout or downloaded crate is needed for these commands from the repository root:

```console
cargo fmt --all --check
cargo build --offline --locked
cargo clippy --offline --locked --all-targets -- -D warnings
cargo test --offline --locked
cargo doc --offline --locked --no-deps
```

`Cargo.lock` is checked in. Do not regenerate it to bypass a locked-build failure.
The initial lock contains only this package. When external dependencies arrive, pin and
review them and revise the offline setup instructions to identify the required cache.

Build and lint validate the interface declarations. **There are no runtime implementations,
unit tests or conformance tests yet.** A successful `cargo test` with zero tests is only
a scaffold check; the acceptance cases below are specifications, not executed evidence.
`cargo doc` produces local API documentation under `target/doc/`.

Run the existing repository checks too:

```console
py check_license.py
py scripts/private_material_scan.py
py scripts/docs_linkcheck.py
gitleaks dir . --config .gitleaks.toml --no-banner --redact --exit-code 1
git diff --check
```

Use `python` or `python3` if the `py` launcher is unavailable. The secret command scans
the working tree; the existing hygiene workflow also scans Git history. No local result
is evidence that hosted CI passed.

## Automatic coverage

The new [Rust workflow](../.github/workflows/rust.yml) runs formatting, build, lint, tests
and warning-free API documentation on pushes to main and pull requests. It uses read-only
repository permissions and has no publishing, deployment or provider steps.
The existing [hygiene workflow](../.github/workflows/repo-hygiene.yml) and
[DCO workflow](../.github/workflows/dco.yml) retain their independent checks.

## Required behavioral acceptance cases

These are **not implemented**. Invariant IDs refer to the catalog in
[platform plan revision 4, Appendix C](https://github.com/iokaio/munarium-platform/blob/main/docs/platform-plan.md) and the
[hub catalog](https://github.com/iokaio/munarium-platform/blob/main/README.md#the-invariant-catalog). No contract bundle has been released.

| Invariant | Scenario | Required observation |
|---|---|---|
| INV-01 / INV-13 | Let a governed writer or delegated proposer attempt ratification. | Refused; ordinary credentials cannot activate governance. |
| INV-09 | Change any authority-bearing request detail after approval. | Old approval is invalid. |
| INV-01 / INV-02 | Ratify successfully but fail activation or race expected prior state. | No false effective status; conflict/failure remains visible. |
| INV-16 | Approve via CLI and later via Console. | Same governed API, role, binding, and attribution checks. |
| INV-20 | Label a new weakening policy as rollback or local override. | No arbitrary rollback or waiver of mandatory parent constraints. |

INV-21 (protected development authority) and INV-22 (claims bounded by evidence)
apply to every packet in addition to the component-specific cases.

## Evidence to retain when the tests exist

Record source and contract digests, toolchain, fixture identifiers, command/exit status,
environment, declared trust boundary, expected and actual outcome, and remaining gaps.
Concurrency, crash/restart, identity, network and storage claims require their real test
environment; an in-memory fake cannot certify them. A live integration needs its own
authorization and qualification record.

Keep operational credentials and raw private payloads out of test artifacts. Distinguish
a local pass, unavailable coverage, a failing case, and an independently reviewed result.
No capability-status or invariant-evidence field advances from the scaffold checks.
