# Security

Do not file a vulnerability as an issue or a pull request.

Report a suspected vulnerability in anything in this repository privately, by either route:

- GitHub's private vulnerability reporting ("Report a vulnerability" under the Security tab), or
- email to **info@ioka.io** with "security" in the subject.

Say what you found, where, and how to reproduce it. Do not include live credentials, customer data,
or a proof of concept run against a system you do not operate. You will get an acknowledgement
within two business days, and a fix, or a recorded decision, on the affected path before any related
release. Credit is given if you ask for it.

## Supported versions

Munarium Council has no release. Until the first tagged release, `main` is the only line and a fix
lands there. Once releases exist, security fixes go to the current minor release and to the previous
one for six months after its successor ships; an older release gets a fix only where the
vulnerability is in a contract it still speaks.

A finding in the design is welcome now, through the same private channel if it has security
consequences and as an ordinary issue otherwise. The threat model this component is built against
is in [README.md](README.md) and, for the platform as a whole, in the hub
([iokaio/munarium-platform](https://github.com/iokaio/munarium-platform)).

## What matters most here

As runtime behavior is implemented, these are the classes of finding taken most seriously and most
quickly:

- **An approval that survives a change.** A changed recipient, amount, artifact, target, policy epoch or target precondition must invalidate the approval; an approval that still applies is the finding this component exists to prevent.
- **A status string accepted as authority.** An ITSM ticket, a chat notification or a portal click is a presentation surface; only the authenticated callback to Council, with the approver, role, decision, request binding, freshness and any required quorum verified, is the authoritative action.
- **A principal ratifying what policy identifies as its own proposal**, or an agent-derived identity becoming the ratifier by delegating through another service.
- **A ratified artifact shown as effective when its activation failed**, or two activations racing without a compare-and-set on the expected current state.
- **The bootstrap attestation path used beyond its permitted transition classes**, after its retirement condition, or without its audit record.
- **Break-glass or rollback widened into a bypass**: an agent suspending its own governance, or an arbitrary new bundle labelled a rollback.

## What is deliberate, and is not a defect

- **Council cannot supply independent two-human review inside a one-person organization.** Where a release or deployment requires that quorum, an external reviewer or customer authority participates, or the corresponding claim stays unqualified. The software supports multiple people; the founder's own operation does not pretend to contain them.
- **The bootstrap attestation is an explicit temporary path, not a bypass.** Before Council is qualified, Server's governance boundary accepts only a narrowly defined attestation from an owner-provisioned, non-agent authority key; its identity, permitted transition classes, expiry or retirement condition and audit record are part of the installation, and a recorded trust transition retires it. Its existence is the design.
- **Shadow evaluation is an impact report, not proof.** It replays a declared set of historical proposals under a candidate and reports changed decisions; a successful shadow run does not prove correctness outside the evaluated set.

When a local development profile exists, its test identity provider, test broker, disposable target
and generated sample credentials are development conveniences confined to that profile. They are
not vulnerabilities in themselves. A path by which they reach a production deployment unnoticed is.

## Findings that cross components

A contract ambiguity that lets two components disagree about authority, a canonicalization
difference between clients, or a gap between what a release advertises and what its evidence
supports is still a security finding. Report it here, or to any other Munarium repository, through
the same private channel; it is routed to the hub and the affected repositories together. Do not
open a public issue for it in the hub.

## Secrets

If you have committed a token or key, treat it as compromised: rotate it first, then report it.
