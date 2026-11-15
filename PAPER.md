# The Borrow Checker as a Binary Auditor: Discovering Multi-Generation Android Firmware Defects via `#![forbid(unsafe_code)]` Safe Rust Reconstruction

**Author:** M. Larrabee (`PNWcoastie`), Independent Systems & Security Researcher  
**Date:** November 2026  
**Document Classification:** Public Research Release (Post-Coordinated-Disclosure Edition)  
**Target Ecosystems Evaluated:**

* **Baseline Generation (2018 / Android 10):** Samsung Galaxy S9 (`SM-G960U` / `starqltesq`, Qualcomm Snapdragon 845 `SDM845`, Firmware `G960USQU9FVB2`)  
* **Modern Production Flagship Generation (2025 to 2026 / Android 16, One UI 8):**  
  * Samsung Galaxy S25 Ultra (`SM-S938B`, Firmware `BP4A.251205.006.S938BXXU9CZDP`)  
  * Samsung Galaxy S26 Ultra (`SM-S948B`, Firmware `BP4A.251205.006.S948BXXS2AZE2`)

---

# Abstract

Prebuilt native shared libraries (`.so` ELF binaries compiled from proprietary C and C++ source trees) form the computational backbone of modern mobile operating systems, powering camera image signal processing (ISP), portrait depth segmentation, human pose tracking, sensor digital signal processing (DSP), and biometric authentication. Because these libraries compile cleanly under standard C/C++ toolchains and pass nominal functional quality assurance on well-formed inputs, mobile OEMs frequently carry prebuilt `.so` binaries (or their underlying C++ utility classes) unchanged across six to eight years of flagship device generations.

This paper introduces **Type-Driven Binary Auditing via `#![forbid(unsafe_code)]` Safe Rust Reconstruction**, a systematic methodology that uses the Rust compiler's strict type, bounds, lifetime, and concurrency rules as a static and differential vulnerability discovery oracle against stripped or semi-stripped `aarch64` binaries. Unlike source-level formal verification or C-to-Rust AST transpilation tools that require original source repositories, this approach operates directly on closed-source vendor binaries. By reconstructing 11 complete native hardware, imaging, sensor, and biometric subsystems (23 ELF64 binaries, 33 computational kernels) from a baseline Samsung Galaxy S9 (`SM-G960U`, Android 10) into modular `#![forbid(unsafe_code)]` Safe Rust and verifying bit-exact mathematical equivalence across 331,853 SplitMix64 and boundary FFI test vectors (plus 15,000 on-silicon Kryo 385 hardware vectors), we uncovered 20 deterministic C/C++ implementation and memory-safety defects (`SAM-BUG-01` through `SAM-BUG-20`) that compiled silently in C/C++.

Cross-generation cryptographic (`SHA-256`), ELF `BuildID`, and `.dynsym` signature auditing against 2026 Android 16 (`BP4A.251205.006`) flagship firmware on the Samsung Galaxy S25 Ultra (`SM-S938B`) and Samsung Galaxy S26 Ultra (`SM-S948B`) revealed that multiple affected libraries continue to ship in `/system/system/lib64/` and `/vendor/lib64/`. These include 100% bit-for-bit identical prebuilt ELF binaries (`libsecimaging_pdk.camera.samsung.so`, `BuildID: 99f17243c5343a20d06f6c9726c9ad6b`, and `libhumantracking_util.camera.samsung.so`, `BuildID: 35a43804f658f688dc0931d9b161e1bd`) carried forward unmodified across flagship generations.

---

# 1. Introduction & Motivation: Why Traditional C/C++ Toolchains and Decompilers Miss "Fossil" Binary Bugs

## 1.1 The Silent Permissiveness of C/C++ Compilation

In C and C++, a wide class of spatial, temporal, arithmetic, and concurrency hazards remains legal at compile time:

1. **Silent 32-Bit Signed Overflow Before 64-Bit Allocation (`CWE-190` → `CWE-680`):** Writing `malloc(width * height * 3)` or `malloc((width * height) << 2)` where `width` and `height` are 32-bit signed `int` (`int32_t`) variables causes the AArch64 compiler to emit a 32-bit multiply (`mul w8, w9, w8`) followed by a 32-to-64-bit sign-extension (`sxtw x0, w8`) into `malloc(size_t)`. No compiler warning is emitted under `-Wall -Wextra`.  
2. **Pointer Decay Without Capacity Bounds (`CWE-120` / `CWE-125`):** Passing a raw pointer `const uint8_t* jpegBuf` or `uint8_t* outBuf` without an accompanying `size_t capacity` parameter is idiomatic C/C++.  
3. **Unchecked Struct Array Indexing (`CWE-787`):** Indexing an inline fixed-size struct array `float samples[25][3]` with a runtime bound `window_size = 50` compiles to unchecked base-plus-offset pointer arithmetic (`this + 12 * write_index`), silently overwriting adjacent struct members (`pthread_mutex_t` and heap pointers) at runtime.  
4. **Control-Flow Misordering (`CWE-476`) and Register Writeback Bugs (`CWE-416`):** Calling `strlen(mStorePath)` prior to `if (mStorePath == nullptr)` or misapplying AArch64 pre-indexed register writeback (`ldr x8, [x19, #0x30]!` followed by `str xzr, [x19, #0x30]`) produces valid ELF machine code.

## 1.2 Why Interactive Decompilers (Ghidra / Hex-Rays) Obscure These Flaws

Traditional binary auditing relies on human analysts reading decompiled C pseudocode in Hex-Rays or Ghidra. However, C decompilers are engineered to reconstruct C-like syntax, re-introducing the very C implicit conversions and pointer decays that hid the bug during compilation:

* A decompiler renders `mul w8, w9, w8; add w8, w8, w8, lsl #1; sxtw x0, w8; bl malloc` as `malloc(width * height * 3)`, hiding the 32-bit truncation and sign-extension boundary unless the analyst inspects raw AArch64 `w` versus `x` register widths.  
* A decompiler renders `readDataFile(const char* path, uint8_t* outBuf)` as a normal function call, without flagging that the caller passed a 1,024-byte buffer while the callee permits reads up to 524,288 bytes (512 KiB).

## 1.3 The Prebuilt Binary Fossil Problem in Mobile Firmware Supply Chains

Mobile flagship firmware integrates dozens of user-space `.so` libraries from internal platform development kits (PDKs), regional R&D centers such as Samsung Research India Bangalore (SRIB), and third-party computer vision vendors such as ArcSoft. When a library's C/C++ ABI remains stable across Android releases, build systems frequently carry the prebuilt `.so` binary (or recompile the legacy C++ utility class without modification) into new flagship devices for years. While recent industry advances in AI-assisted source scanning and open-source formal verification (such as proofs over open-source cryptographic messaging libraries) require source-tree access, proprietary vendor blobs remain opaque to source-only pipelines.

---

# 2. Methodology: The Borrow Checker as a Binary Auditor

Instead of treating decompiled C pseudocode as the final representation for human review, our methodology uses compilation into `#![forbid(unsafe_code)]` Safe Rust paired with closed-loop differential FFI parity testing as a deterministic verification filter:

1. **Stage 1 (Whole-Subgraph AArch64 Extraction):** Extract connected ELF64 `.dynsym` call subgraphs, packed `APS2` relocations, C++ vtables, and byte-exact struct layouts.  
2. **Stage 2 (`#![forbid(unsafe_code)]` Safe Rust Port):** Reconstruct the subgraph in Safe Rust, where the compiler rejects implicit width conversions, raw pointer dereferences, unchecked struct indexing, and unsynchronized globals.  
3. **Stage 3 (Closed-Loop Differential Parity):** Verify bit-exact equivalence across 331,853 SplitMix64 and boundary FFI vectors against reference oracles (`0 ULP` tolerance).  
4. **Stage 4 (Cross-Generation ELF Continuity):** Audit cryptographic digests (`SHA-256`), ELF `BuildID` metadata, and `.dynsym` signatures from Android 10 (`SM-G960U`) forward into Android 16 (`SM-S938B` and `SM-S948B`).

## 2.1 Stage 1: Whole-Subgraph AArch64 Extraction & Struct Recovery

For each target `.so` binary, we extract complete connected call subgraphs rather than isolated snippets:

* **Relocation & Symbol Resolution:** We resolve standard `.rela.plt` entries alongside Android's packed `APS2` (`SHT_ANDROID_RELA`) signed-LEB128 relocations, recovering C++ vtables (`.data.rel.ro`), `.rodata` IEEE-754 filter tables, and `.bss` global state layouts.  
* **Exact Struct Field Mapping:** Every `[x0, #imm]` load and store across constructors, methods, and destructors is mapped to a byte-exact struct table (`sizeof`, alignment, field types, and embedded synchronization primitives such as 40-byte Bionic `pthread_mutex_t` and 48-byte `pthread_cond_t`).

## 2.2 Stage 2: The Four `#![forbid(unsafe_code)]` Rust Compiler Rejection Mechanisms

When translating a recovered AArch64 subgraph into `#![forbid(unsafe_code)]` Rust, four classes of latent C/C++ binary defects become impossible to express without explicit handling, forcing the auditor to confront every unsafe assumption in the original binary:

| Mechanism | What C/C++ & AArch64 Silently Allow | Why `#![forbid(unsafe_code)]` Rust Rejects It | Bugs Exposed in Audit |
| :--- | :--- | :--- | :--- |
| **1. Width & Sign-Extension Enforcement** | `int32_t` (`w` reg) arithmetic sign-extended (`sxtw`) into 64-bit `size_t` (`x` reg) for `malloc` or pointer offsets | Rust forbids implicit `i32` → `usize` conversions (`TryFrom` or explicit `.checked_mul()` required; negative `i32` cannot index a slice or allocate a `Vec`) | `SAM-BUG-04`, `SAM-BUG-12`, `SAM-BUG-14`, `SAM-BUG-16`, `SAM-BUG-20` |
| **2. Mandatory Slice Fat-Pointers (`&[u8]`)** | Raw pointers (`const uint8_t*`) passed across API boundaries without buffer lengths | Dereferencing `*const u8` is forbidden; every buffer must be a fat pointer `&[u8]` (`ptr` + `len`). If the binary ABI lacks `len`, a safe slice cannot be constructed | `SAM-BUG-13`, `SAM-BUG-15`, `libFaceService` `readDataFile` |
| **3. Checked Struct Array Bounds (`[T; N]`)** | Out-of-bounds loop indices (`0..50` or `0..face_count`) overwriting adjacent struct fields on the heap or stack | Indexing `samples[i]` on `[[f32; 3]; 25]` or `faces[i]` on `[Face; 5]` inserts a mandatory bounds check (`i < N`) | `SAM-BUG-01` (`Filter(50)` into `[25]`), `SAM-BUG-06` (`fd_array[32]`), `SAM-BUG-14` (`face_count > 5`) |
| **4. Ownership & Sync Invariants** | Unsynchronized `.bss` globals (`g_effectWrapper`, DSP filter states) and dangling pointers after `Uninit` | `static mut` is forbidden under `#![forbid(unsafe_code)]`; state must be owned by an instance struct or wrapped in `Mutex`/`OnceLock`, and handles use `Option` / RAII | `SAM-BUG-02` (`mHandle` writeback), `SAM-BUG-11` (20 `.bss` DSP globals), `SAM-BUG-17` (`g_effectWrapper` UAF) |

## 2.3 Stage 3: Closed-Loop Mathematical Parity & On-Silicon Verification

To prove that our `#![forbid(unsafe_code)]` Rust reconstructions capture the exact mathematical semantics of the original binaries on valid inputs (while isolating the exact boundary inputs where the C/C++ code overflows or faults), we executed closed-loop differential testing across all 11 subsystems (33 kernels):

* **331,853 Deterministic FFI Test Vectors:** Every kernel was tested against 10,000+ SplitMix64 pseudo-random and boundary vectors (`0 ULP` floating-point tolerance), achieving 100% bit-exact parity (`0` counterexamples) and up to 6.36× throughput speedup over the baseline C reference oracles.  
* **Live On-Silicon Verification (`SM-G960U` Kryo 385 ARM64):** A static `#![forbid(unsafe_code)]` ARM64 verification binary (`s9_live_verify`) was executed directly on the physical Samsung Galaxy S9 (`SDM845`) across 15,000 on-device calls (15 kernels), producing bit-identical 64-bit FNV-1a digests (`0 ULP` difference) between `x86_64` and physical `aarch64` silicon.

### Summary Parity & Performance Scorecard Across All 11 Reconstructed Subsystems

| Subsystem | Source Binary (`aarch64` ELF) | Safe Rust Crate (`#![forbid(unsafe_code)]`) | Kernels | Total Vectors | Parity Status | Peak Speedup vs. C |
| :--- | :--- | :--- | :---: | :---: | :--- | :---: |
| **1. Blur Detection** | `libBlurDetection.camera.samsung.so` | `s9_blur_detection` | 3 | 30,118 | `verified_bit_exact (0 ULP)` | 2.66× |
| **2A. Camera Gyroscope** | `libgyroscope.camera.samsung.so` | `s9_camera_gyroscope` | 3 | 30,155 | `verified_bit_exact (0 ULP)` | 1.33× |
| **2B. Sensor Service DSP** | `libsensorservice.so` | `s9_sensor_dsp` | 3 | 30,155 | `verified_bit_exact (0 ULP)` | 2.15× |
| **2C. Iris 850nm NIR** | `libIrisTlc.so` + `iris.default.so` | `s9_iris_nir` | 3 | 30,155 | `verified_bit_exact (0 ULP)` | 2.12× |
| **2D. Hexagon FastRPC & ION** | `lib{s,a,c}dsprpc.so` + `gralloc.sdm845.so` | `s9_hexagon_fastrpc` | 3 | 30,155 | `verified_bit_exact (0 ULP)` | 1.24× |
| **3A. Camera Sensor Wrapper** | `libSDSensorWrapper` + `libsemcamera_jni` | `s9_camera_sensor_wrapper` | 3 | 30,192 | `verified_bit_exact (0 ULP)` | 1.39× |
| **3B. Human Pose Tracking** | `libhumantracking_util.camera.samsung.so` | `s9_human_tracking` | 3 | 30,192 | `verified_bit_exact (0 ULP)` | 1.47× |
| **3C. Iris Biometric Service** | `libIrisSensorListener.so` + `libIrisService.so` | `s9_iris_service` | 3 | 30,192 | `verified_bit_exact (0 ULP)` | 1.59× |
| **3D. Face Biometric HAL** | `face.default.so` + `libFaceService.so` | `s9_face_hal` | 3 | 30,192 | `verified_bit_exact (0 ULP)` | 6.36× |
| **4A. Smart Imaging & PDK** | `libsecimaging.so` + `libsecimaging_pdk.so` | `s9_sec_imaging` | 3 | 30,192 | `verified_bit_exact (0 ULP)` | 4.80× |
| **4B. SRIB Depth & OutFocus** | `libsrib_outfocus` + `libsrib_humanaware_engine` | `s9_srib_depth_segmentation` | 3 | 30,155 | `verified_bit_exact (0 ULP)` | 2.41× |
| **Total** | **23 Native `.so` Binaries** | **11 Safe Rust Crates (0 `unsafe` blocks)** | **33** | **331,853** | **100% Bit-Exact** | **Up to 6.36×** |

---

# 3. Taxonomy of the 20 Discovered C/C++ Binary Defects (`SAM-BUG-01` .. `SAM-BUG-20`)

Every defect cataloged below was identified during the Stage 1 and Stage 2 reconstruction of the `SM-G960U` baseline binaries and subsequently audited for upstream survival in Android 16 (`SM-S938B` and `SM-S948B`):

| Bug ID | CWE Class | Affected Library | Affected C/C++ Symbol & Offset (`SM-G960U` / `SM-S948B`) | Technical Summary |
| :--- | :--- | :--- | :--- | :--- |
| **SAM-BUG-01** | `CWE-787 (Heap/Struct OOB Write)` | `libgyroscope.camera.samsung.so` | `SecGyroCalibration::Init` (`0x3260`) → `Filter::Filter(int)` (`0x3a90`) | Allocates `sizeof(Filter) = 0x180` (384 B) with inline `float samples[25][3]` (300 B at `+0x00..+0x12b`), but passes hardcoded `window_size = 50`. Samples `25..49` overwrite `window_size`, `count`, `write_index`, `scratch_vec` pointer, `pthread_mutex_t`, and 224 B past the heap chunk. |
| **SAM-BUG-02** | `CWE-416 (Dangling Handle / UAF)` | `libhumantracking_util.camera.samsung.so` | `ArcSoftHumanTrackingHelper::unInitializeHumanTracking` (`0x04108`) | Pre-indexed writeback `ldr x8, [x19, #0x30]!` shifts `x19` (`this`) to `this + 0x30`; subsequent `str xzr, [x19, #0x30]` writes `0` to `this + 0x60` (`mJointMap11[0]`) instead of zeroing `this->mHandle` (`this + 0x30`). |
| **SAM-BUG-03** | `CWE-121 (Stack Buffer Overflow)` | `libFaceService.so` | `FaceService::request` (`cmd_id == 11`, `0x04c44..0x04c68`) | Formats arbitrary 32-bit signed integer via `__vsprintf_chk(..., "%d")` into an 8-byte stack slot (`sp + 0x08..0x0f`); negative 32-bit values (`"-2147483648"`, 12 bytes with NUL) overflow the 8-byte slot. (Note: instruction `0xeb2842df` decodes to extended-register `subs xzr, x22, w8, sxtw #0`, which acts as `cmp`.) |
| **SAM-BUG-04** | `CWE-191 / CWE-121 (Underflow to Stack OOB)` | `libFaceService.so & libIrisService.so` | `FaceServiceStorage::setStorePath` (`0x078dc`) | Computes `copy_len = strlen(path) - 3` and calls `strncpy(stack_buf_100, path, copy_len)`. If `strlen(path) < 3`, `copy_len` underflows to `SIZE_MAX - 2..SIZE_MAX`; if `> 103`, overflows the 100-byte stack buffer. |
| **SAM-BUG-05** | `CWE-476 (NULL Dereference Before Check)` | `libFaceService.so` | `FaceService::authenticate` (`0x0479c`) & `migrateInterimTemplate` (`0x04fd0`) | Calls `bl strlen` on `mStorePath` before executing the `cbz x20` (`if (mStorePath == nullptr)`) guard. |
| **SAM-BUG-06** | `CWE-121 (Stack Buffer Overflow)` | `libsemcamera_jni.camera.samsung.so` | `JNISemCameraContext::_setOutputFileArray` (`0x0922c`) | Copies JNI `jintArray` elements into fixed `int fd_array[32]` on stack (`sp + 0x10..0x90`) without checking `GetArrayLength <= 32`. |
| **SAM-BUG-07** | `CWE-770 (Stack Exhaustion)` | `libFaceService.so & libIrisService.so` | `FaceService::migrateInterimTemplate` (`0x04f74`) | Allocates 525,408 B (`sub sp, sp, #0x80000`) directly on the calling Binder thread stack (1 MiB limit). |
| **SAM-BUG-08** | `CWE-476 / CWE-457 (Null Deref & Uninit Stack)` | `libIrisSensorListener.so` | `LightSensorListener::getSensorData` (`0x01d28`) | Dereferences `float* out_lux` without null check and allocates 832 B (`ASensorEvent[8]`) on stack without zero-initialization. |
| **SAM-BUG-09** | `CWE-675 (Duplicate Resource Op)` | `libIrisService.so` | `IrisWorkerManager::processEnroll` (`0x19d80`) | Invokes `IrisServiceStorage::storeTemplate` twice back-to-back with identical arguments when `progress == 100`. |
| **SAM-BUG-10** | `CWE-252 / CWE-369 (Unchecked malloc & sdiv)` | `libBlurDetection.camera.samsung.so` | `get_blur_val` (`0x1704`) & `get_cnt_lt` (`0x14d4`) | 5× `malloc(w * h)` calls dereferenced without NULL checks; `sdiv` by `(w/5)*(h/5)` without zero check (`w < 5` or `h < 5`). |
| **SAM-BUG-11** | `CWE-362 (Data Race on .bss)` | `libsensorservice.so` | `ButterworthFilter`, `MakeFilteredAccData`, `AutoGyroBias` (`0x70580..0x76158`) | 20 MRE sensor DSP functions mutate shared `.bss` filter states (`0x78cb0..0x78ed0`) without mutex or atomic synchronization. |
| **SAM-BUG-12** | `CWE-190 (Integer Overflow to memcpy)` | `libIrisTlc.so` | `IrisTlc_GetPreviewFrame_NV21` (`0x08430`) | Computes `width * height * 3 / 2` in 32-bit signed arithmetic before passing to `memcpy`. |
| **SAM-BUG-13** | `CWE-125 (Unbounded String Parse)` | `libSDSensorWrapper.camera.samsung.so` | `SDSensorWrapperImpl::setSceneInfo` (`0x0cecc`) | Passes `(char* data, int size)` buffer to `atof`/`atoi` without verifying a NUL terminator within `data[0..size-1]`. |
| **SAM-BUG-14** | `CWE-190 / CWE-125 / CWE-369` | `libhumantracking_util.camera.samsung.so` | `ArcSoftHumanTrackingHelper::processHumanTracking` (`0x03290`) | 102,464-B stack frame + unclamped `face_count` loop over 5-face array + 6× unguarded `sdiv` by `width`/`height` + 32-bit `mul w9, w22, w23; add x9, x25, w9, sxtw` UV pointer offset. |
| **SAM-BUG-15** | `CWE-125 / CWE-197 (OOB Read & Truncation)` | `libsecimaging_pdk.camera.samsung.so` | `get_SIZE` (`0x1a654` S9 / `0x34dac` S26U) | Scans JPEG marker segments advancing pointer by untrusted 16-bit big-endian lengths `(p[2] << 8) + p[3]`; S26U narrows caller's 64-bit `size_t` (`m`) to 32-bit signed `int` (`i`). |
| **SAM-BUG-16** | `CWE-190 → CWE-680 (Int Overflow to Heap OOB)` | `libsecimaging_pdk.camera.samsung.so & libsecimaging.camera.samsung.so` | `getDecodeBufSize` (`0x34fbc` S26U), `getEncodeBufSize` (`0x34f60` S26U), `SCImage::initialize` | Computes `(width * height) << 2` or `width * height * 3` in 32-bit signed `int32_t` (`w` regs) from 16-bit JPEG `SOF` dimensions (`0..65535`), wrapping positive or negative before heap allocation. |
| **SAM-BUG-17** | `CWE-416 (Concurrent JNI UAF)` | `libjni_dualcamera.so` | `DualCameraEffect_init` / `render` / `release` (`0x00ee8..0x01578`) | Mutates and `delete`s `.bss` global `g_effectWrapper` (`0x20008`) across JNI calls without mutex synchronization. |
| **SAM-BUG-18** | `CWE-401 (Memory Leak)` | `libjni_dualcamera.so` | `DDMNativeEngine_nativeGetDepthMapSize` (`0x01a50`) | Allocates `new int32_t[2]` (`_Znam`) on every query and returns without `delete[]`. |
| **SAM-BUG-19** | `CWE-369 (Divide-by-Zero)` | `libsrib_outfocus.camera.samsung.so` | `si_generateDefocusImage` (`0x5c648`) & `SS_MatrixInverse` (`0x1f1d0`) | Divides RGB accumulators by `max_blur_weight` (`sdiv`) without zero check; divides 3×3 covariance cofactors by `det` without checking `det == 0.0`. |
| **SAM-BUG-20** | `CWE-190 → CWE-680 & CWE-120` | `libsrib_humanaware_engine.camera.samsung.so` | `SRIBHumanAwareController::SetBufferMetaInfo` (`0x8c3d8` S26U) & `IcsThread::C2` (`0xaeef8` S26U) | Multiplies `in_width * in_height * 3` in 32-bit `w8` and sign-extends (`sxtw x0, w8`) to `malloc(size_t)`; `IcsThread` copies thread name via unbounded `strcpy`. |

---

# 4. Deep-Dive Technical Case Studies (AArch64 Assembly → Safe Rust Rejection → Remediation)

## 4.1 Case Study 1 (`SAM-BUG-15` & `SAM-BUG-16`): 32-Bit Signed Overflow & `size_t` Truncation in `libsecimaging_pdk.camera.samsung.so`

* **Binary Continuity:** `/system/system/lib64/libsecimaging_pdk.camera.samsung.so` is 100% bit-for-bit identical (`SHA-256: de1151f33f7b3ceb987a6744ee84f1f7e1bf228c748f971f1fae7a5af8d8e3e1`, `BuildID: 99f17243c5343a20d06f6c9726c9ad6b`) across the Galaxy S25 Ultra (`SM-S938B`) and Galaxy S26 Ultra (`SM-S948B`) on Android 16.  
* **Exported `.dynsym` Call Chain on `SM-S948B` (Android 16):**

```text
decodeFromMemory (0x36a30, Exported C API)
  └──> _Z14fastJpegDecodePKhmPiS1_i (0x35ecc: takes 64-bit size_t `m` buffer length)
         ├──> _Z8get_SIZEPKhiRiS1_ (0x34dac: truncates 64-bit size_t `m` to 32-bit signed `int` `i`)
         └──> _Z16getDecodeBufSizePKhiii (0x34fbc: computes 32-bit signed `int32_t` buffer size)
```

* **Mathematical Proof of Positive 32-Bit Overflow Bypassing `<= 0` Checks:**  
  `get_SIZE` extracts 16-bit unsigned big-endian `height` and `width` (`0..65535`) from a JPEG `SOF0..SOF15` marker and passes them directly to `getDecodeBufSize(jpegBuf, rawFormat, width, height)` (`0x34fbc`). For `RGBA` / `BGRA` (`rawFormat == 7` or `8`), `getDecodeBufSize` evaluates:

```c
int32_t px = width * height;
int32_t buf_size = px << 2; // 32-bit signed arithmetic in w registers
```

For `width = 32768` (`0x8000`) and `height = 32769` (`0x8001`):

```text
px = 32768 * 32769 = 1,073,774,592 = 2^30 + 32768
buf_size = (px << 2) mod 2^32 = (2^32 + 131,072) mod 2^32 = +131,072 bytes (+128 KiB)
```

Because `+131,072` is a positive, non-zero `int32_t`, it bypasses any `if (buf_size <= 0)` check and allocates a 128 KiB heap buffer for an image whose true decoded size is 4,295,098,368 bytes (~4 GiB). Decoding just 2 scanlines (256 KiB) writes 128 KiB past the end of the heap buffer.

* **How `#![forbid(unsafe_code)]` Rust Exposed It:**  
  In Safe Rust (`s9_sec_imaging`), an allocation size must have type `usize`. Multiplying two `i32` values (`width * height`) produces an `i32`, which the Rust compiler refuses to pass to `vec![0u8; size]` without an explicit `usize::try_from(...)` and `.checked_mul(4)`. Remediating this in C++ requires widening `get_SIZE` and `getDecodeBufSize` to `size_t` and using `__builtin_mul_overflow`.

## 4.2 Case Study 2 (`SAM-BUG-20`): 32-Bit `w * h * 3` `sxtw` Sign-Extension to `malloc` in `libsrib_humanaware_engine.camera.samsung.so`

* **Binary Continuity:** Ships in both `/system/system/lib64/` and `/vendor/lib64/` on `SM-S948B` (`BuildID: da180f48c6f46f679498ada04e888d6386f18f02`) and `SM-S938B` (`BuildID: 44d9108eac3de50579877818daa7eb0f14ae92cc`).  
* **AArch64 Instruction Sequence in `_ZN24SRIBHumanAwareController17SetBufferMetaInfoEN15srib_humanaware14BufferMetaInfoE` (`0x8c3d8` on S26U):**

```armasm
mul  w8, w9, w8             ; w8 = in_width * in_height (32-bit signed multiply)
add  w8, w8, w8, lsl #1     ; w8 = w8 * 3 (32-bit wrapping addition)
sxtw x0, w8                 ; Sign-extends 32-bit w8 to 64-bit size_t for malloc!
bl   malloc
```

* **Mathematical Proof:**  
  For `in_width = 37283` and `in_height = 38400`, the true 3-channel byte size is `37283 * 38400 * 3 = 4,294,992,000` bytes. Evaluated modulo `2^32` in 32-bit `w8`, this wraps to `+24,704` bytes (24 KiB). `sxtw x0, w8` sign-extends `+24,704` into `x0`, causing `malloc(24704)` to succeed and return an undersized 24 KiB heap buffer into `this->packed_input_buf` (`+0x10`).

## 4.3 Case Study 3 (`SAM-BUG-02` & `SAM-BUG-14`): Pre-Indexed Register Writeback (`!`) Clobbering `this + 0x60` in `libhumantracking_util.camera.samsung.so`

* **Binary Continuity:** `/system/system/lib64/libhumantracking_util.camera.samsung.so` is 100% bit-for-bit identical (`SHA-256: 8888620156c2b4c10dfb1032f7c7093751a447230f38b40ae7caa8450d891f3c`, `BuildID: 35a43804f658f688dc0931d9b161e1bd`) across `SM-S948B` S26 Ultra and `SM-S938B` S25 Ultra.  
* **AArch64 Instruction Sequence in `_ZN7android26ArcSoftHumanTrackingHelper25unInitializeHumanTrackingEv`:**

```armasm
ldr  x8, [x19, #0x30]!   ; Loads this->mHandle AND increments x19 by +0x30 (x19 is now this + 0x30)
cbz  x8, .Ldone
mov  x0, x19             ; Passes &this->mHandle (this + 0x30) to ARC_HT_Uninit
bl   ARC_HT_Uninit
str  xzr, [x19, #0x30]   ; BUG: Because x19 was already shifted by +0x30, [x19, #0x30] writes 0 to (this + 0x60)!
```

* **Root Cause & Rust Prevention:**  
  In `ArcSoftHumanTrackingHelper`, `mHandle` lives at `this + 0x30` and the joint mapping table `mJointMap11` starts at `this + 0x60`. Because `ldr x8, [x19, #0x30]!` uses pre-indexed writeback (`!`), `x19` is mutated in-place to `this + 0x30`. The subsequent instruction `str xzr, [x19, #0x30]` adds `#0x30` a second time, zeroing `this + 0x60` (`mJointMap11[0]`) instead of `this + 0x30` (`mHandle`). In Safe Rust (`s9_human_tracking`), `self.handle = None` mutates the struct field by name rather than raw register offset, eliminating offset-aliasing bugs.

## 4.4 Case Study 4 (`SAM-BUG-04` & `SAM-BUG-05`): Unbounded `readDataFile` and Pre-Check `strlen(mStorePath)` in `libFaceService.so`

* **Binary Continuity:** `/vendor/lib64/libFaceService.so` on `SM-S948B` (`SHA-256: ecf7f86bf5324b96ec3790010f15553f8d73ecd4e060a37ecc245bdd951b08fc`, `BuildID: 3b8dad5d726f48e437a742563dab4896`) and `SM-S938B` (`SHA-256: 4658be6e89c4b74a470b300c1e0213a386a3efbd7625978ba880d5a4b0c99fb8`, `BuildID: fdee7b92a0a19e1fbbb79ecc030b5667`).  
* **ABI Proof in `.dynsym` (`SM-S948B`):**

```text
00000000000438e0 T _ZN7android18FaceServiceStorage12readDataFileEPKcPh     // (const char* path, uint8_t* outBuf) : NO size_t len!
0000000000042b08 T _ZN7android18FaceServiceStorage14readDataFileAtEPKclPvm // (const char* path, long offset, void* outBuf, size_t len)
00000000000430a4 T _ZN7android18FaceServiceStorage11getFileSizeEPKc        // Permits files up to 512 KiB (0x80000)
0000000000043dfc T _ZN7android18FaceServiceStorage20retrieveSecureUserIDEv // Passes 1 KiB (0x400) sid.dat buffer to readDataFile
```

* **How `#![forbid(unsafe_code)]` Rust Exposed It:**  
  In Safe Rust (`s9_face_hal`), a file-reading method must take `out_buf: &mut [u8]`. Because a Rust slice always carries `out_buf.len()`, it is impossible to write a safe `read_data_file` function that ignores the destination buffer's capacity and trusts `getFileSize(path)` (`<= 512 KiB`) when writing into a 1 KiB `sid.dat` buffer.

## 4.5 Case Study 5 (`SAM-BUG-01`): Hardcoded `new Filter(50)` Overwriting a 25-Element Inline Struct Array in `libgyroscope.camera.samsung.so`

* **Struct Layout Recovery (`sizeof(Filter) == 0x180 = 384` bytes):**

```c
struct Filter {
    /* +0x000..+0x12b */ float           samples[25][3]; // 25 * 12 = 300 bytes (0x12c)
    /* +0x12c         */ int32_t         window_size;    // Set to 50 by SecGyroCalibration::Init!
    /* +0x130         */ int32_t         count;
    /* +0x138         */ int64_t         write_index;    // Advances 0..49
    /* +0x140         */ float*          scratch_vec;    // Heap pointer overwritten at sample 26!
    /* +0x148         */ float           average[3];
    /* +0x154..+0x17b */ pthread_mutex_t mutex;          // Overwritten at samples 28..31!
};
```

* **Root Cause & Companion Crate (`s9_camera_gyroscope`):**  
  `SecGyroCalibration::Init()` (`0x3260`) allocates `0x180` bytes via `operator new` and passes the literal `w1 = 50` (`#0x32`) to `Filter::Filter(int)`. Once `Filter::movingAverage` processes sample 25, `this + 12 * write_index` steps directly out of `samples[25][3]` (`+0x12b`) into `window_size` (`+0x12c`), `scratch_vec` (`+0x140`), `pthread_mutex_t` (`+0x154`), and 224 bytes of adjacent heap memory. In the companion `#![forbid(unsafe_code)]` Safe Rust crate [`s9_camera_gyroscope`](./src/lib.rs), `Filter<const N: usize>` enforces compile-time array capacity (`[[f32; 3]; N]`) with `idx = self.write_index % N`, completely eliminating the 25-sample struct corruption hazard while preserving `0 ULP` bit-exact mathematical parity across 30,155 test vectors.

---

# 5. Cross-Generation Firmware Supply-Chain Continuity (`SM-G960U` Android 10 → `SM-S938B` & `SM-S948B` Android 16)

To test our hypothesis that unpatched C/C++ camera and biometric libraries persist across flagship generations, we compared the `SM-G960U` (Android 10) binaries against 27 binaries extracted from 2026 Android 16 (`BP4A.251205.006`) production firmware on the Galaxy S25 Ultra (`SM-S938B`) and Galaxy S26 Ultra (`SM-S948B`):

| Library Name | `SM-G960U` (Android 10) SHA-256 | `SM-S938B` S25 Ultra (Android 16) SHA-256 & BuildID | `SM-S948B` S26 Ultra (Android 16) SHA-256 & BuildID | Cross-Generation Finding |
| :--- | :--- | :--- | :--- | :--- |
| **`libsecimaging_pdk.camera.samsung.so`** | `e9d225de82a96ee4...` (406 KB) | `de1151f33f7b3ceb...` (612 KB), `BuildID: 99f17243c5343a20d06f6c9726c9ad6b` | `de1151f33f7b3ceb...` (612 KB), `BuildID: 99f17243c5343a20d06f6c9726c9ad6b` | 1:1 Bit-Identical between S25U & S26U; retains `_Z16getDecodeBufSizePKhiii` (`0x34fbc`) & `_Z8get_SIZEPKhiRiS1_` (`0x34dac`) |
| **`libsecimaging.camera.samsung.so`** | `b83b1ebe713b6a89...` (754 KB) | `5691e13e11f18f04...` (855 KB), `BuildID: 3f235b504b64b1ca70285a87baf84d2d` | `c9e9d51274963676...` (855 KB), `BuildID: b75e25803d3730b132b07ec487fe66c2` | Retains `SFilterBufferedProcessorImpl::processBufferArray` (`0x4ad28`, `0x4b624`) |
| **`libhumantracking_util.camera.samsung.so`** | `c376bd673aa6b66d...` (39 KB) | `8888620156c2b4c1...` (51 KB), `BuildID: 35a43804f658f688dc0931d9b161e1bd` | `8888620156c2b4c1...` (51 KB), `BuildID: 35a43804f658f688dc0931d9b161e1bd` | 1:1 Bit-Identical between S25U & S26U; retains `ArcSoftHumanTrackingHelper` bugs (`SAM-BUG-02`, `SAM-BUG-14`) |
| **`libFaceService.so`** | `9013ac8b926b3724...` (140 KB) | `4658be6e89c4b74a...` (486 KB), `BuildID: fdee7b92a0a19e1fbbb79ecc030b5667` | `ecf7f86bf5324b96...` (486 KB), `BuildID: 3b8dad5d726f48e437a742563dab4896` | Retains unbounded `_ZN7android18FaceServiceStorage12readDataFileEPKcPh` (`0x438e0`) alongside bounded `readDataFileAt` (`0x42b08`) |
| **`libsrib_humanaware_engine.camera.samsung.so`** | `ec6d87e41512394c...` (1.1 MB) | `dbc6123fda3af700...` (2.4 MB), `BuildID: 44d9108eac3de50579877818daa7eb0f14ae92cc` | `c76e99ef7ecbfa3e...` (2.4 MB), `BuildID: da180f48c6f46f679498ada04e888d6386f18f02` | Ships identically in `/system/system/lib64/` and `/vendor/lib64/`; retains `SetBufferMetaInfo` (`0x8c3d8`) & `IcsThread` (`0xaeef8`) |
| **`libsensorservice.so`** | `d2c1be5ae67528df...` (628 KB) | `4ec8ec57eecb01f5...` (750 KB), `BuildID: 225821b6f4a8e741be3fc2f538a4ab68` | `d8ea7a7f687a5e2c...` (750 KB), `BuildID: abe3454c0f33221355c993615d0a7d4b` | Retains unsynchronized `.bss` MRE sensor DSP globals (`SAM-BUG-11`) |

---

# 6. Threat Model, Stock Retail Device Boundary Verification, and Scientific Limitations

## 6.1 Stock Unrooted Retail Device Verification (`Knox 0x0`, `SELinux Enforcing`)

All live dynamic verification and binary cross-checks were performed on a stock, locked-bootloader retail device with Samsung Knox intact (`Warranty Bit: 0x0`) and `SELinux` in `Enforcing` mode from a standard unprivileged context (`uid=2000`, `u:r:shell:s0`), and cross-verified against unmodified stock production firmware for the Galaxy S25 Ultra (`SM-S938B`, `S938BXXU9CZDP`) and Galaxy S26 Ultra (`SM-S948B`, `S948BXXS2AZE2`). Because the affected `.so` libraries and their exported symbols (`decodeFromMemory`, `fastJpegDecode`, `get_SIZE`, `getDecodeBufSize`, `SRIBEngineSetBufferMetaInfo`, etc.) reside in accessible library paths (`/system/system/lib64/` and `/vendor/lib64/`) with default world-read/execute permissions (`0644`/`0755`), they can be mapped and reached on locked, Knox-enforced production devices without root or bootloader unlocking.

## 6.2 Distinction Between Deterministic Source/Binary Defects and IPC Privilege Escalation

In accordance with vulnerability research standards, we explicitly distinguish between:

1. **Deterministic Binary & Source Code Defects (Proven 100% in This Study):** Every cataloged bug (`SAM-BUG-01` .. `SAM-BUG-20`) is a concrete arithmetic, bounds, lifetime, or synchronization flaw proven directly in the shipping `aarch64` machine code and `.dynsym` ABI.  
2. **End-to-End Privilege Boundary Exploitation (Out of Scope / Future Work):** In the Android security model, dynamically loading (`dlopen`) a shared library inside an unprivileged caller's own process crashes or corrupts only the caller's own address space ("Self-DoS"). Demonstrating remote or local privilege escalation requires showing that a privileged daemon (`cameraserver`, `system_server`, or biometric HAL service) invokes the vulnerable entrypoint on untrusted Binder IPC or media inputs without prior caller-side dimension clamping. We intentionally scoped this research to defensive root-cause identification and upstream C/C++ source remediation rather than offensive exploit development.

---

# 7. Ethics, Safe-Harbor Scope, and Coordinated Vulnerability Disclosure

1. **Defensive Security Research Purpose:** All binary analysis and `#![forbid(unsafe_code)]` Rust modeling were performed exclusively for defensive vulnerability discovery, memory-safety verification, and source remediation reporting under DMCA Section 1201(j) good-faith security research principles.  
2. **Zero Redistribution of Proprietary Binaries:** No proprietary OEM `.so` binaries, firmware images, or commercial derivative drivers are redistributed with this whitepaper. Only cryptographic digests (`SHA-256`), ELF `BuildID` metadata, short fair-use disassembly excerpts (`5 to 10` instructions) necessary to illustrate the mathematical defect, and defensive C/C++ remediation recommendations are included.  
3. **Coordinated Disclosure Timeline:** A consolidated upstream engineering report and C/C++ source remediation patchset covering the Android 16 flagship findings (`SM-S948B` and `SM-S938B`) was privately submitted to Samsung Mobile Security (PSRT) on October 20, 2026 (`Report I-124838`). PSRT reviewed and closed `Report I-124838` without upstream source remediation under Samsung's September 2026 policy requiring end-to-end privilege-escalation exploit PoCs rather than static and differential root-cause proofs. Following vendor closure, this defensive methodology and non-weaponized root-cause analysis are published to advance memory-safe binary auditing research.

---

# 8. Related Work & Prior-Art Differentiation

To situate this contribution within the systems security and programming languages literature, we contrast our methodology against four adjacent research thrusts: (1) automated C-to-Rust source migration, (2) platform memory-safety adoption in Android, (3) binary lifting and decompilation, and (4) empirical Android vendor-blob lifecycle studies.

## 8.1 Automated C-to-Rust Source Translation (`DARPA TRACTOR`, `C2Rust`, `Laertes`, `CROWN`, `C2SaferRust`, `Syzygy`, `ForCLift`)

A major body of recent research, led by [DARPA's TRACTOR (Translating All C to Rust)](https://www.darpa.mil/research/programs/translating-all-c-to-rust) program (including the *ForCLift* verified-lifting coalition) and academic systems including [C2Rust](https://c2rust.com/) (Immunant & Galois), Laertes (Emre et al., *OOPSLA 2021*), CROWN (Zhang et al., *CAV 2023*), C2SaferRust (Nitin et al., *FSE 2024*), Syzygy (Shetty et al., *ICSE/LLM4Code 2025*), and CRUST-Bench (Khatry et al., 2025), focuses on forward source-to-source translation.

* **Prior-Art Scope:** These systems require access to the original C/C++ source code (`compile_commands.json`, ASTs, header files) and aim to replace legacy C source repositories with compiled Rust code. Syntactic transpilers like `C2Rust` emit `unsafe` raw-pointer Rust (`*mut T`, `libc` calls) that intentionally preserves the exact memory-unsafety semantics (and bugs) of the underlying C code so the transpiled program compiles without refactoring. Follow-on pointer-lifting tools (`Laertes`, `CROWN`, `C2SaferRust`) measure success by the fraction of raw pointers automatically promoted to safe references in open-source C benchmarks.  
* **How Our Methodology Differs:**  
  1. **Closed-Source Binary Input vs. Open-Source AST Input:** We operate on compiled, stripped `aarch64` proprietary shared libraries (`.so`) where C/C++ source code is unavailable outside the OEM.  
  2. **The Compiler as a Vulnerability Oracle vs. Code Replacement:** Even when an OEM does not ship Rust in its production firmware, we use `#![forbid(unsafe_code)]` Safe Rust reconstruction as an external static and differential vulnerability discovery oracle. Every place where the `aarch64` binary's control flow or ABI cannot be expressed in `#![forbid(unsafe_code)]` Rust without introducing an explicit checked cast, bounds guard, capacity slice, or synchronization wrapper directly pinpoints a latent defect in the vendor's upstream C/C++ source code.

## 8.2 Google's Android Memory-Safety Strategy & C++/Rust Interoperability

Google's Android security and platform teams (Vander Stoep & Rebert, *Google Online Security Blog 2024*; Bergstrom, *2024*; Vander Stoep & Android Security Team, *2025*) have demonstrated that prioritizing memory-safe languages (Rust) for newly written AOSP code yields an exponential decline in memory-safety vulnerabilities, reducing vulnerability density by 1,000× and driving memory-safety bugs below 20% of total Android vulnerabilities without rewriting the entire legacy C/C++ codebase. Parallel investments in `Crubit` and `autocxx` focus on hardening source-level FFI boundaries between new Rust components and existing C++ code.

* **How Our Methodology Complements & Extends This Work:**  
  Google's "Safe Coding" model governs newly authored code and source-level FFI wrappers. However, OEM vendor partitions (`/vendor/lib64/` and proprietary `/system/system/lib64/` libraries) contain hundreds of legacy, prebuilt C/C++ shared libraries that are neither newly written in Rust nor open to AOSP source-level FFI wrapping. Our work demonstrates how Rust's safety model can be applied retroactively and externally to legacy C/C++ binaries: by combining `#![forbid(unsafe_code)]` binary reconstruction with differential FFI testing (331,853 vectors), security auditors can systematically extract and remediate latent C/C++ memory-safety debt inside closed-source vendor blobs that otherwise bypass AOSP's shift-left Rust protections.

## 8.3 Binary Lifting and Decompilation (`Oxidizer`, LLVM IR Lifters)

Traditional binary lifters (`McSema`, `RetDec`, `rev.ng`) lift machine code into LLVM IR or C pseudocode for recompilation or coverage-guided fuzzing, while recent decompilation research such as Oxidizer (Liu et al., *IEEE S&P 2026*) recovers high-level Rust idioms from compiled Rust binaries.

* **How Our Methodology Differs:**  
  Decompiling C/C++ binaries back into C pseudocode (via Ghidra or IDA Hex-Rays) preserves C's permissive semantics, meaning implicit `int32_t` → `size_t` sign-extensions (`sxtw`), pointer-only buffer arguments, and struct offset aliasing appear benign in the decompiled C output. Conversely, rather than decompiling Rust binaries (*Oxidizer*), we lift compiled C/C++ binaries into `#![forbid(unsafe_code)]` Safe Rust, exploiting the semantic gap between C/assembly and Safe Rust as a bug-finding lens.

## 8.4 Empirical Studies of Stale Android Vendor Blobs (*Keep Me Updated*)

Recent large-scale empirical work, notably Wen et al. (*"Keep Me Updated: An Empirical Study of Proprietary Vendor Blobs in Android Firmware"*, *IEEE ICPADS 2024*) and Maar et al. (*"The Doom of Device Drivers: Your Android Device (Most Likely) Has N-Day Kernel Vulnerabilities"*, *USENIX Security 2024*), established that 82% of Android firmware releases ship outdated prebuilt vendor blobs lagging up to 1,281 days behind upstream codebases.

* **How Our Methodology Differs:**  
  Prior vendor-blob studies measure staleness at a macro metadata level (tracking build timestamps, statically linked LLVM versions, or known N-day CVE signatures). Our study unites micro-architectural 0-day vulnerability discovery (via `#![forbid(unsafe_code)]` reconstruction and 331,853-seed differential FFI verification on a 2018 Android 10 baseline) with cross-generation ELF `BuildID`, `SHA-256`, and `.dynsym` continuity tracking into 2025 and 2026 Android 16 flagships (`SM-S938B` and `SM-S948B`). This proves not only that vendor blobs are stale (`1:1` bit-identical `BuildID`s across S25 Ultra and S26 Ultra), but also exposes the exact unpatched arithmetic, bounds, and register-writeback defects carried forward inside those blobs across six to eight years of flagship hardware.

| Research Category | Representative Prior Art | Primary Input | Primary Goal | Uses `#![forbid(unsafe_code)]` as Binary Bug-Finding Oracle? | Tracks Cross-Gen ELF BuildID / `.dynsym` Continuity? |
| :--- | :--- | :--- | :--- | :---: | :---: |
| **C-to-Rust Source Transpilation** | `DARPA TRACTOR`, `C2Rust`, `Laertes`, `CROWN`, `C2SaferRust`, `Syzygy` | Open-source C AST / `compile_commands.json` | Replace C source codebase with Rust | No (targets source migration) | No |
| **Platform Rust Adoption** | Vander Stoep & Rebert (2024), Bergstrom (2024), Android Rust (2025) | New AOSP source code & C++ headers (`Crubit`) | Prevent bugs in newly written platform code | No (forward engineering) | No |
| **Rust Binary Decompilation** | `Oxidizer` (*IEEE S&P 2026*) | Compiled Rust binary | Recover readable Rust from Rust binaries | No | No |
| **Vendor Blob Staleness Studies** | Wen et al. (*Keep Me Updated*, *ICPADS 2024*); Maar et al. (*USENIX Security 2024*) | Firmware `.so` metadata & known CVEs | Measure update lag & N-day persistence | No | Macro version/timestamp level |
| **This Work** | **Safe Rust Reconstruction as a Binary Audit Oracle** | **Stripped C/C++ `aarch64` vendor `.so` binaries** | **Discover latent C/C++ binary defects & verify cross-gen survival** | **Yes (331,853 FFI parity vectors across 11 crates)** | **Yes (Android 10 `SM-G960U` → Android 16 `SM-S938B` & `SM-S948B`)** |

---

# 9. References

* **Immunant & Galois (2024).** *C2Rust: Migrate Legacy C Code into Rust* ([c2rust.com](https://c2rust.com/)) and [DARPA TRACTOR Program](https://www.darpa.mil/research/programs/translating-all-c-to-rust).  
* **Emre, M., Schroeder, R., Dewey, K., & Hardekopf, B. (2021).** "Translating C to Safer Rust." *Proceedings of the ACM on Programming Languages (PACMPL)*, Vol. 5, Issue OOPSLA.  
* **Zhang, H., David, C., Yu, Y., & Wang, M. (2023).** "Ownership Guided C to Rust Translation (CROWN)." *International Conference on Computer Aided Verification (CAV)*.  
* **Nitin, V., Krishna, R., Valle, L. L., & Ray, B. (2024).** "C2SaferRust: Transforming C Projects into Safer Rust with NeuroSymbolic Techniques." *ACM International Conference on the Foundations of Software Engineering (FSE)*.  
* **Shetty, M., Jain, N., Godbole, A., Seshia, S. A., & Sen, K. (2025).** "Syzygy: Dual Code-Test C to (safe) Rust Translation using LLMs and Dynamic Analysis." *ICSE Workshop on Foundation Models and Software Engineering (LLM4Code)*.  
* **Khatry, A., Zhang, R., Pan, J., et al. (2025).** "CRUST-Bench: A Comprehensive Benchmark for C-to-safe-Rust Transpilation." *arXiv preprint*.  
* **Vander Stoep, J., & Rebert, A. (2024).** "Eliminating Memory Safety Vulnerabilities at the Source." *Google Online Security Blog*, September 2024.  
* **Bergstrom, L. (2024).** "Improving Interoperability Between Rust and C++." *Google Open Source & Security Blog* / Rust Foundation, February 2024.  
* **Vander Stoep, J., & Android Security Team (2025).** "Rust in Android: Move Fast and Fix Things." *Google Online Security Blog*, November 2025.  
* **Liu, Y., Basque, Z. L., Raj, A. S., et al. (2026).** "Oxidizer: Toward Concise and High-fidelity Rust Decompilation." *IEEE Symposium on Security and Privacy (S&P)*.  
* **Wen, E., Shen, J., & Wünsche, B. (2024).** "Keep Me Updated: An Empirical Study of Proprietary Vendor Blobs in Android Firmware." *IEEE 30th International Conference on Parallel and Distributed Systems (ICPADS)*.  
* **Maar, L., Draschbacher, F., Schumm, L., Martínez García, E., & Mangard, S. (2024).** "The Doom of Device Drivers: Your Android Device (Most Likely) Has N-Day Kernel Vulnerabilities." *33rd USENIX Security Symposium (USENIX Security 24)*.
