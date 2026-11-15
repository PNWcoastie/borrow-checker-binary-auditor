# The Borrow Checker as a Binary Auditor (`s9_camera_gyroscope`)

[![Language: Safe Rust](https://img.shields.io/badge/rust-%23!%5Bforbid(unsafe__code)%5D-brightgreen.svg)](https://www.rust-lang.org/)
[![Verification: 30,155 FFI Vectors](https://img.shields.io/badge/verify__rust__parity-30%2C155%20vectors%20(0%20ULP)-blue.svg)](./docs/STAGE3_4_PARITY_SCORECARD.md)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-informational.svg)](./LICENSE-MIT)

Official companion artifact and `#![forbid(unsafe_code)]` Safe Rust reference crate for the public research paper:

> **The Borrow Checker as a Binary Auditor: Discovering Multi-Generation Android Firmware Defects via `#![forbid(unsafe_code)]` Safe Rust Reconstruction**  
> **Author:** M. Larrabee (`PNWcoastie`), Independent Systems & Security Researcher (November 2026)  
> 📄 **[Read the Full Research Paper (`PAPER.md`)](./PAPER.md)**

---

## Overview

This repository provides a complete, self-contained demonstration of **Type-Driven Binary Auditing via `#![forbid(unsafe_code)]` Safe Rust Reconstruction** on `libgyroscope.camera.samsung.so` (`SHA-256: d6a0bfb60a8352bb4029b4e39a99f280f8592693fa8e46734cc9547794f3ee97`), the 416Hz Electronic Image Stabilization (EIS) quaternion integrator and temperature-binned gyroscope drift compensation DSP library from the Samsung Galaxy S9 (`SM-G960U` / `starqltesq`, Android 10):

1. **[`PAPER.md`](./PAPER.md):** Complete research paper detailing the 4-stage methodology, the 20 discovered C/C++ binary defects (`SAM-BUG-01` through `SAM-BUG-20`) across 11 subsystems (23 ELF64 binaries, 331,853 FFI parity vectors), and cross-generation ELF `BuildID` / `.dynsym` continuity into 2025–2026 Android 16 flagships (`SM-S938B` & `SM-S948B`).
2. **[`src/lib.rs`](./src/lib.rs):** Clean-room, zero-dependency `#![forbid(unsafe_code)]` Safe Rust reconstruction (`s9_camera_gyroscope`) implementing:
   - `Quat` and `Quat::mult_by_quat` (`0x47f0`): Left-associative IEEE-754 Hamilton product composition.
   - `RotationVector` and `RotationVector::process_algorithm` (`0x3e10`): 416Hz attitude quaternion integrator with exact mixed `f32` → `f64` → `f32` precision promotions and `0.005 rad/s` axis-normalization gating.
   - `get_rotation_matrix_from_vector` (`0x43d0`): 4×4 row-major rotation matrix and 3-axis Euler angle extractor with `±0.49999` gimbal-lock singularity guards.
   - `Filter<const N: usize>` (`0x3a90..0x3d98`): Compile-time bounds-checked 3-axis moving average and variance filter that eliminates **`SAM-BUG-01`** (Section 4.5 Case Study 5), where `SecGyroCalibration::Init` (`0x3260`) passes `window_size = 50` into a `Filter` struct containing only a 25-element (`300`-byte) inline `float samples[25][3]` array (`+0x00..+0x12b`), overwriting `window_size`, `count`, `write_index`, `scratch_vec`, `pthread_mutex_t`, and 224 bytes past the heap allocation.
   - `calculate_offset_linear_fitting` (`0x2f70`) & `is_device_not_in_motion_gyro` (`0x34f0`): Least-squares thermal drift regression and 11-sample stationary variance detector (`< 1.0e-5`).
3. **[`docs/STAGE1_SUBGRAPH_AUDIT.md`](./docs/STAGE1_SUBGRAPH_AUDIT.md):** Stage 1 whole-subgraph AArch64 disassembly, `.rodata` IEEE-754 constants, C++ struct offset tables, and reference C implementations.
4. **[`docs/STAGE3_4_PARITY_SCORECARD.md`](./docs/STAGE3_4_PARITY_SCORECARD.md):** Stages 3 & 4 closed-loop differential FFI verification scorecard across `30,155` SplitMix64 and boundary test vectors (`0 ULP` divergence).
5. **[`tests/parity_regression.rs`](./tests/parity_regression.rs):** Deterministic C-oracle bit-exact regression test suite and `SAM-BUG-01` `Filter<50>` bounds-safety proof.

---

## Differential FFI Verification Summary (`s9_camera_gyroscope`)

| Kernel Symbol | Arg Types | Return Type | Parity Status | Vectors Tested | C Oracle (`ns/call`) | Safe Rust (`ns/call`) | Speedup vs. C |
| :--- | :--- | :--- | :--- | :---: | :---: | :---: | :---: |
| [`quat_mult_w`](./src/lib.rs#L309-L319) | `["f32"; 8]` | `f32` | **`verified_bit_exact` (`0 ULP`)** | `10,064` | `5.93 ns` | `4.46 ns` | **`1.33×`** |
| [`rotation_vector_step_w`](./src/lib.rs#L324-L337) | `["f32"×3, "i64", "f32"×4]` | `f32` | **`verified_bit_exact` (`0 ULP`)** | `10,064` | `14.93 ns` | `17.05 ns` | `0.88×` |
| [`gyro_calibration_pipeline`](./src/lib.rs#L343-L376) | `["bytes"]` | `f32` | **`verified_bit_exact` (`0 ULP`)** | `10,027` | `634.72 ns` | `611.44 ns` | **`1.04×`** |

---

## Quickstart: Build, Test, Lint & Docs

```bash
# Run unit tests and deterministic C-oracle parity regression vectors
cargo test

# Enforce zero compiler or Clippy warnings across all targets
cargo clippy --all-targets -- -D warnings

# Generate rustdoc documentation
cargo doc --no-deps
```

---

## Citation & Zenodo Archival

If you reference this methodology, paper, or artifact in academic or security research, please cite [`CITATION.cff`](./CITATION.cff):

```bibtex
@misc{larrabee2026borrowchecker,
  author       = {Larrabee, Matthew},
  title        = {{The Borrow Checker as a Binary Auditor: Discovering Multi-Generation Android Firmware Defects via \#![forbid(unsafe\_code)] Safe Rust Reconstruction}},
  year         = {2026},
  month        = nov,
  publisher    = {Zenodo},
  url          = {https://github.com/PNWcoastie/borrow-checker-binary-auditor}
}
```

## License & Ethics Statement

- **Code & Documentation License:** Dual-licensed under [MIT](./LICENSE-MIT) or [Apache-2.0](./LICENSE-APACHE) at your option.
- **Zero Proprietary Binary Redistribution:** No proprietary OEM `.so` binaries or firmware images are included in this repository. All code is a 100% clean-room `#![forbid(unsafe_code)]` Safe Rust implementation developed for defensive security research and interoperability under DMCA Section 1201(j).
