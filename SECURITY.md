# Security Policy

ZisK is a zkVM: its job is to make sure that a valid proof can only be produced for a correct execution. We take reports of anything that undermines that guarantee seriously, and we appreciate the work of researchers who find and disclose such issues responsibly.

ZisK is in alpha and is currently undergoing security and correctness audits.

## Supported Versions

While ZisK is in alpha, security fixes go into the version under development and are shipped in the next release. Older releases are not patched.

| Version             | Supported |
| ------------------- | --------- |
| Latest release      | ✅        |
| Earlier releases    | ❌        |

The next version is developed on the latest `pre-develop-<version>` branch. Before reporting, please check whether the issue is still present there: it may already have been fixed.

## Reporting a Vulnerability

**Please do not report security vulnerabilities through public GitHub issues, pull requests or discussions.**

Report them privately through GitHub instead:

1. Go to the [Security tab](https://github.com/0xPolygonHermez/zisk/security) of this repository.
2. Click **Report a vulnerability**, or use [this link](https://github.com/0xPolygonHermez/zisk/security/advisories/new) directly.

Only the reporter and the ZisK maintainers can see the report.

Please include as much of the following as you can:

- The affected component (for example a state machine or PIL file, a precompile, the verifier or the Solidity contracts).
- The version or commit SHA you tested, and the file and line where the problem is.
- A description of the issue and its impact.
- A proof of concept: for a soundness issue, a statement the verifier accepts although it is false, or a witness that satisfies the constraints without corresponding to a correct execution; for other issues, steps to reproduce.
- A suggested fix, if you have one.

## What to Expect

- We will acknowledge your report as soon as possible.
- We will send an initial assessment, including whether we consider it a vulnerability and how severe it is.
- We will keep you informed while we work on a fix, and tell you when it is released.

Once a fix is available, we publish a GitHub security advisory describing the issue. We credit reporters in the advisory unless they ask to remain anonymous. Please keep the issue confidential until the advisory is published.

**There is no Bug Bounty Programme at the moment.**

## Scope

In scope are issues in the code in this repository that let someone:

- Produce a proof that verifies for an execution that did not happen, or for a wrong output (soundness), for example through missing or incorrect constraints in the PIL state machines or precompiles.
- Make the verifier, the recursion pipeline or the Solidity verifier contracts (`zisk-contracts`) accept an invalid proof.
- Make the emulator or executor deviate from the RISC-V semantics in a way that a proof would attest to.
- Compromise users who install or run ZisK, for example through the installer (`ziskup`) or the build tooling.

Out of scope:

- Vulnerabilities in third-party dependencies that do not affect ZisK (please report those upstream; if ZisK is affected, report it to us as well).
- Crashes, slowdowns or failures to produce a proof for a valid execution, unless they have a security impact (please open a regular issue for those).
- Issues that require an already compromised machine.

If you are unsure whether something is in scope, report it privately anyway.
