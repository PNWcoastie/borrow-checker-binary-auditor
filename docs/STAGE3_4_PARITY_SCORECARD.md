# Stage 3 & 4 — Closed-Loop Differential FFI Verification & Executive Scorecard (`libgyroscope.camera.samsung.so` → `s9_camera_gyroscope`)

## 1. Executive Summary

- **Target Artifact:** `/system/lib64/libgyroscope.camera.samsung.so` (SHA-256: `d6a0bfb60a8352bb4029b4e39a99f280f8592693fa8e46734cc9547794f3ee97`, `"GYRODRIFT :: Library Version - 1.20.3 Release Date - 06-March-19"`)
- **Reconstructed Safe Rust Crate:** [`s9_camera_gyroscope`](../src/lib.rs)
- **Deterministic Regression Suite:** [`tests/parity_regression.rs`](../tests/parity_regression.rs)
- **Verification Engine:** `verify_rust_parity` (`harness_mode: "native-rustc"`, `seed: 1337420`, `float_ulp_tolerance: 0`, `forbid_unsafe: true`)
- **Overall Parity Verdict:** **`verified_bit_exact`** (`0` ULP divergence across `30,155` total randomized SplitMix64 + boundary FFI test vectors and `300,000` benchmark invocations)

---

## 2. Differential FFI Verification & Benchmark Telemetry (`verify_rust_parity`)

| Kernel Symbol | Evidence ID | Arg Types | Return Type | Status | Random + Boundary Vectors | `#![forbid(unsafe_code)]` / Unsafe Blocks | C Oracle (`ns/call`) | Safe Rust (`ns/call`) | Speedup vs. C Oracle | Sandbox Backend (`wall_time_ms`) |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| [`quat_mult_w`](../src/lib.rs#L309-L319) | `ev_9f0bdbed52930cdf6a8de8efb361c1c8f4d29196a8baac9833517a61aef34cf1` | `["f32", "f32", "f32", "f32", "f32", "f32", "f32", "f32"]` | `f32` | **`verified_bit_exact`** (`0` ULP) | `10,000 + 64 = 10,064` | `true` / `0` | `5.9251 ns` | `4.4634 ns` | **`1.33×`** | `linux-bwrap+prlimit` (`12 ms`) |
| [`rotation_vector_step_w`](../src/lib.rs#L324-L337) | `ev_9181b0378aad62d0587c849b84aa3d967ab29bed8984c3c0c173e5772abe6b24` | `["f32", "f32", "f32", "i64", "f32", "f32", "f32", "f32"]` | `f32` | **`verified_bit_exact`** (`0` ULP) | `10,000 + 64 = 10,064` | `true` / `0` | `14.9337 ns` | `17.0516 ns` | **`0.88×`** | `linux-bwrap+prlimit` (`22 ms`) |
| [`gyro_calibration_pipeline`](../src/lib.rs#L343-L376) | `ev_937d2b595fad53b369ab79b70b61a756b757f8a64b6572ab03270a8c7bbc336f` | `["bytes"]` | `f32` | **`verified_bit_exact`** (`0` ULP) | `10,000 + 27 = 10,027` | `true` / `0` | `634.7222 ns` | `611.4429 ns` | **`1.04×`** | `linux-bwrap+prlimit` (`151 ms`) |

---

## 3. C-to-Rust Mathematical Invariant & Memory-Safety Mapping Table

| AArch64 Binary Function (`VA`) | C/C++ Reference Behavior & Latent Hazards | Safe Rust Reconstruction (`src/lib.rs`) | Verified Mathematical & Memory Invariant |
| :--- | :--- | :--- | :--- |
| `_ZN7android4Quat10multByQuatEPS0_S1_` (`0x47f0`) | Computes Hamilton product `out = lhs * this` ($q \otimes d$) with left-associative IEEE-754 `f32` `fmul`/`fadd`/`fsub` trees (`ow = ((qw * dw - dx * qx) - dy * qy) - dz * qz`, etc.) via raw `Quat*` pointers. | [`Quat::mult_by_quat`](../src/lib.rs#L65-L73), [`quat_mult_w`](../src/lib.rs#L309-L319) | Value-type `Quat` struct with identical left-associative parenthesization; verified `0`-ULP bit-exact across `10,064` vectors (`1.33×` speedup). |
| `_ZN7android14RotationVector16ProcessAlgorithmENS_3vecIfLm3EEEix` (`0x3e10`) | Integrates `sensor_type == 4` gyro samples: computes `dt_f32 = (float)(timestamp_ns - prev_timestamp_ns) / 1.0e9f`, promotes `dt_f64 = (double)dt_f32`, computes `mag = sqrtf((gx*gx + gy*gy) + gz*gz)`, uses threshold `0.005f` (`0x3ba3d70a`) to select `(gx/mag, gy/mag, gz/mag)` vs. `(gx, gy, gz)`, evaluates `sin`/`cos` of `(dt_f64 * (double)mag) * 0.5` in `f64`, sets `delta_q.w = -(float)cos(half_theta)`, and updates `current_quat` via `multByQuat`. | [`RotationVector::process_algorithm`](../src/lib.rs#L97-L127), [`rotation_vector_step_w`](../src/lib.rs#L324-L337) | Preserves the exact mixed `f32` → `f64` → `f32` precision promotions, threshold `0.005_f32`, and negated `w = -(c as f32)` convention; verified `0`-ULP bit-exact across `10,064` vectors. |
| `_ZN7android12GyroscopeLib27getRotationMatrixFromVectorEPfNS_3vecIfLm4EEES1_` (`0x43d0`) | Builds 4×4 row-major rotation matrix using `neg_q3 = -q3` off-diagonal products and extracts 3-axis Euler angles with `f64` gimbal-lock guards at `±0.49999 * sq_norm` (`0x4c30`, `0x4c38`). Writes into raw `float*` buffers without bounds checks. | [`get_rotation_matrix_from_vector`](../src/lib.rs#L134-L170) | Returns strongly typed `([f32; 16], [f32; 3])` arrays on the stack; verified `0`-ULP bit-exact inside `gyro_calibration_pipeline`. |
| `_ZN7android6FilterC1Ei` (`0x3a90`), `movingAverage` (`0x3b20`), `getVariance` (`0x3cf0`) | Allocates `sizeof(Filter) = 0x180` (`384` bytes) with only `300` bytes (`25` × 12-byte `vec3` slots) before `window_size` at `+0x12c`. Calling `new Filter(50)` in `SecGyroCalibration` (`0x3324`) overflows `samples[25..49]` into `window_size`, `count`, `write_index`, `average`, and `pthread_mutex_t` (`+0x154`) (`SAM-BUG-01`)! | [`Filter<const N: usize>`](../src/lib.rs#L175-L248) | Const-generic `[[f32; 3]; N]` array guarantees compile-time capacity `N` (`Filter<3>`, `Filter<11>`, `Filter<50>`) with `idx = self.write_index % N`, completely eliminating the 25-sample struct corruption hazard while preserving `0`-ULP moving-average and variance parity. |
| `_ZN7android18SecGyroCalibration24IsDeviceNotInMotion_GyroENS_3vecIfLm3EEE` (`0x34f0`) & `_ZN7android15CalculateOffset13LinearFittingEv` (`0x2f70`) | Tracks 11-sample magnitude variance against `1.0e-5` (`0x4c28`) with a 10-sample stationary hysteresis counter, and computes least-squares temperature drift slope/intercept (`cov_xy / var_x`). Uses `.bss` global pointers (`0x7008..0x7028`) and detached `pthread_create` threads without synchronization. | [`is_device_not_in_motion_gyro`](../src/lib.rs#L281-L304), [`calculate_offset_linear_fitting`](../src/lib.rs#L252-L276) | Pure owned state with zero `.bss` globals or detached threads, explicit `var_x == 0.0` zero-division guard, and `0`-ULP bit-exact equivalence across `10,027` vectors (`1.04×` speedup). |
