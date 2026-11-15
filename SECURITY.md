# Security & Responsible Disclosure Policy

## Clean-Room Research Artifact Boundary

This repository contains the open-access research paper (**[PAPER.md](PAPER.md)**) and the clean-room, `#![forbid(unsafe_code)]` Safe Rust reconstruction crate (`s9_camera_gyroscope`) accompanying:

> **The Borrow Checker as a Binary Auditor: Discovering 20 Latent Memory, Logic, and ABI Bugs in Stripped Android Vendor Binaries via `#![forbid(unsafe_code)]` Rust Reconstruction**

To uphold strict legal, ethical, and coordinated vulnerability disclosure standards:

1. **No Proprietary Binary Redistribution:** This repository contains **zero** proprietary OEM shared libraries (`.so`), firmware images, or copyrighted vendor code. All binary provenance is documented strictly via cryptographic digests (`SHA-256`), ELF `BuildID`s, symbol/offset metadata, and clean-room Safe Rust reconstructions.
2. **No Weaponized Exploit Payloads:** Do **not** submit Pull Requests, Issues, or Discussions containing weaponized Proof-of-Concept (`PoC`) exploit chains, privilege-escalation payloads, or unredacted memory dumps targeting commercial devices.
3. **Reporting Vulnerabilities in Third-Party Firmware:** If you use the `#![forbid(unsafe_code)]` reconstruction methodology described in [PAPER.md](PAPER.md) to discover new vulnerabilities in third-party vendor binaries, please report them directly to the affected vendor's Product Security Incident Response Team (PSIRT) — for example, [Samsung Mobile Security](https://security.samsungmobile.com/) or the [Android Security Bounty Program](https://bughunters.google.com/) — under standard 90-day Coordinated Vulnerability Disclosure (CVD) guidelines before public discussion.

## Reporting Issues in This Crate

If you discover a mathematical discrepancy, soundness issue, or documentation erratum in the `s9_camera_gyroscope` Safe Rust crate or [PAPER.md](PAPER.md), please open a structured issue via the **[Crate Reproduction or Paper Errata](../../issues/new?template=reproduction_or_errata.yml)** template.
