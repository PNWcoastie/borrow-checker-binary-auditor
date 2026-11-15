# Stage 1 — Camera-Gyroscope 416Hz Drift Compensation & 3D Rotation Matrix DSP Specification (`libgyroscope.camera.samsung.so`)

## 1. Binary Overview & Segment Layout

- **Device / Firmware Path:** `/system/lib64/libgyroscope.camera.samsung.so` (`SM-G960U` / `starqltesq`, Firmware `G960USQU9FVB2`)
- **SHA-256 Digest:** `d6a0bfb60a8352bb4029b4e39a99f280f8592693fa8e46734cc9547794f3ee97`
- **Library Version String (`.rodata` `0x4f33`):** `"GYRODRIFT :: Library Version - 1.20.3 Release Date - 06-March-19"`
- **Format / Architecture:** `ELF64` (`AArch64` / `arm64`, little-endian, Itanium C++ ABI)
- **Segments & Key Sections:**
  - `LOAD_0`: `0x0000..0x5894` (`22,676` bytes)
    - `.dynsym` (`0x0238..0x0c58`), `.dynstr` (`0x0c58..0x18b5`)
    - `.rela.dyn` (`0x1c80..0x1d58`), `.rela.plt` (`0x1d58..0x23a0`), `.plt` (`0x23a0..0x27f0`)
    - `.text` (`0x27f0..0x4c1c`)
    - `.rodata` (`0x4c20..0x4fb5`)
  - `LOAD_1`: `0x6b20..0x7030` (`1,296` bytes)
    - `.got` (`0x6d90..0x6dd0`), `.got.plt` (`0x6dd0..0x7000`)
    - `.data` (`0x7000..0x7008`), `.bss` (`0x7008..0x7030`)

---

## 2. Exported C++ Classes & Method Signatures

| Address | Mangled Symbol | Recovered C++ Signature |
| :--- | :--- | :--- |
| `0x28c0` | `_ZN7android15CalculateOffsetC2Ev` | `android::CalculateOffset::CalculateOffset()` |
| `0x2960` | `_ZN7android15CalculateOffset13initAlgorithmEv` | `void android::CalculateOffset::initAlgorithm()` |
| `0x2ad0` | `_ZN7android15CalculateOffsetD2Ev` | `android::CalculateOffset::~CalculateOffset()` |
| `0x2c50` | `_ZN7android15CalculateOffset12verAlgorithmEf` | `void android::CalculateOffset::verAlgorithm(float residual)` |
| `0x2d50` | `_ZN7android15CalculateOffset14resetAlgorithmEv` | `void android::CalculateOffset::resetAlgorithm()` |
| `0x2d90` | `_ZN7android15CalculateOffset6InsertEff` | `void android::CalculateOffset::Insert(float gyro_sample, float temperature)` |
| `0x2f30` | `_ZN7android15CalculateOffset7getmeanEf` | `float android::CalculateOffset::getmean(float temperature)` |
| `0x2f70` | `_ZN7android15CalculateOffset13LinearFittingEv` | `void android::CalculateOffset::LinearFitting()` |
| `0x3020` | `_ZN7android15CalculateOffset9GetOffsetEf` | `float android::CalculateOffset::GetOffset(float temperature)` |
| `0x30f0` | `_ZN7android27ThreadRunnableLinearFittingEPv` | `void* android::ThreadRunnableLinearFitting(void* arg)` |
| `0x31f0` | `_ZN7android18SecGyroCalibrationC2Ev` | `android::SecGyroCalibration::SecGyroCalibration()` |
| `0x33a0` | `_ZN7android18SecGyroCalibration14getTemperatureEv` | `void android::SecGyroCalibration::getTemperature()` |
| `0x3410` | `_ZN7android18SecGyroCalibrationD2Ev` | `android::SecGyroCalibration::~SecGyroCalibration()` |
| `0x34f0` | `_ZN7android18SecGyroCalibration24IsDeviceNotInMotion_GyroENS_3vecIfLm3EEE` | `bool android::SecGyroCalibration::IsDeviceNotInMotion_Gyro(android::vec<float, 3> gyro)` |
| `0x35f0` | `_ZN7android18SecGyroCalibration7processENS_3vecIfLm3EEEix` | `void android::SecGyroCalibration::process(android::vec<float, 3> gyro, int sensor_type, int64_t timestamp)` |
| `0x38e0` | `_ZN7android20getTemperatureThreadEPv` | `void* android::getTemperatureThread(void* arg)` |
| `0x3a80` | `_ZN7android18SecGyroCalibration8RegisterEPNS_8callbackE` | `void android::SecGyroCalibration::Register(android::callback* cb)` |
| `0x3a90` | `_ZN7android6FilterC1Ei` | `android::Filter::Filter(int window_size)` |
| `0x3af0` | `_ZN7android6FilterD2Ev` | `android::Filter::~Filter()` |
| `0x3b20` | `_ZN7android6Filter13movingAverageEfff` | `android::vec<float, 3> android::Filter::movingAverage(float x, float y, float z)` |
| `0x3ca0` | `_ZN7android6Filter5resetEv` | `void android::Filter::reset()` |
| `0x3cd0` | `_ZN7android6Filter10getAverageEv` | `android::vec<float, 3> android::Filter::getAverage()` |
| `0x3cf0` | `_ZN7android6Filter11getVarianceEv` | `android::vec<float, 3> android::Filter::getVariance()` |
| `0x3da0` | `_ZN7android14RotationVectorC2Ev` | `android::RotationVector::RotationVector()` |
| `0x3de0` | `_ZN7android14RotationVectorD2Ev` | `android::RotationVector::~RotationVector()` |
| `0x3e10` | `_ZN7android14RotationVector16ProcessAlgorithmENS_3vecIfLm3EEEix` | `void android::RotationVector::ProcessAlgorithm(android::vec<float, 3> gyro, int sensor_type, int64_t timestamp)` |
| `0x3fa0` | `_ZN7android14RotationVector8RegisterEPNS_8callbackE` | `void android::RotationVector::Register(android::callback* cb)` |
| `0x3fb0` | `_ZN7android12GyroscopeLibC2Ei` | `android::GyroscopeLib::GyroscopeLib(int mode_mask)` |
| `0x40b0` | `_ZN7android12GyroscopeLibD2Ev` | `android::GyroscopeLib::~GyroscopeLib()` |
| `0x4130` | `_ZN7android12GyroscopeLib11processGyroEiPfx` | `void android::GyroscopeLib::processGyro(int sensor_type, float* gyro_xyz, int64_t timestamp)` |
| `0x4200` | `_ZN7android12GyroscopeLib8RegisterEPNS_15wrapperCallbackE` | `void android::GyroscopeLib::Register(android::wrapperCallback* cb)` |
| `0x4210` | `_ZN7android12GyroscopeLib18getCompensatedGyroEPfix` | `void android::GyroscopeLib::getCompensatedGyro(float* comp_gyro_xyz, int sensor_type, int64_t timestamp)` |
| `0x42f0` | `_ZN7android12GyroscopeLib17getRotationMatrixEPf` | `void android::GyroscopeLib::getRotationMatrix(float* quat_xyzw)` |
| `0x43d0` | `_ZN7android12GyroscopeLib27getRotationMatrixFromVectorEPfNS_3vecIfLm4EEES1_` | `void android::GyroscopeLib::getRotationMatrixFromVector(float* out_R4x4, android::vec<float, 4> quat_xyzw, float* out_euler3)` |
| `0x45b0` | `_ZN7android4QuatC2Effff` | `android::Quat::Quat(float x, float y, float z, float w)` |
| `0x4620` | `_ZN7android4QuatC1Ev` | `android::Quat::Quat()` |
| `0x4670` | `_ZN7android4Quat16loadIdentityQuatEv` | `void android::Quat::loadIdentityQuat()` |
| `0x46c0` | `_ZN7android4Quat5cloneEv` | `android::Quat* android::Quat::clone()` |
| `0x4730` | `_ZN7android4Quat5equalEPS0_S1_` | `bool android::Quat::equal(android::Quat* a, android::Quat* b)` |
| `0x4790` | `_ZN7android4Quat5equalEPS0_` | `bool android::Quat::equal(android::Quat* other)` |
| `0x47f0` | `_ZN7android4Quat10multByQuatEPS0_S1_` | `void android::Quat::multByQuat(android::Quat* lhs, android::Quat* out)` |
| `0x4a00` | `_ZN7android5Vec4fC2Effff` | `android::Vec4f::Vec4f(float x, float y, float z, float w)` |
| `0x4a20` | `_ZN7android5Vec4fC2Ev` | `android::Vec4f::Vec4f()` |
| `0x4a30` | `_ZN7android5Vec4f8copyVec4EPS0_` | `void android::Vec4f::copyVec4(android::Vec4f* src)` |
| `0x4a60`..`0x4aa0` | `Vec4f::setX/Y/Z/W`, `ToArray` | `void setX(float)`, `void setY(float)`, `void setZ(float)`, `void setW(float)`, `float* ToArray()` |
| `0x4ab0` | `_ZN7android13Gyroscope_Lib12processEventEiPfx` | `void android::Gyroscope_Lib::processEvent(int sensor_type, float* gyro_xyz, int64_t timestamp)` |
| `0x4ac0` | `_ZN7android13Gyroscope_Lib13initializeLibEiPNS_12callbackFuncE` | `void android::Gyroscope_Lib::initializeLib(int mode_mask, android::callbackFunc* cb)` |
| `0x4b50` | `_ZN7android13Gyroscope_LibD1Ev` | `android::Gyroscope_Lib::~Gyroscope_Lib()` |

---

## 3. Recovered C++ Class Struct Layouts & Field Offsets

### 3.1 `android::Vec4f` and `android::Quat` (`sizeof = 16` bytes, `0x10`)

```cpp
struct Vec4f {
    float x; // +0x00
    float y; // +0x04
    float z; // +0x08
    float w; // +0x0c
};
struct Quat : public Vec4f {}; // Identity initialized to (0.0f, 0.0f, 0.0f, 1.0f)
```

### 3.2 `android::RotationVector` (`sizeof = 32` bytes, `0x20`)

| Offset | Type | Field Name | Description |
| :--- | :--- | :--- | :--- |
| `+0x00` | `int64_t` | `prev_timestamp_ns` | Previous gyroscope event timestamp in nanoseconds (`0` on first sample) |
| `+0x08` | `double` | `omega_magnitude` | Latest angular velocity norm $\sqrt{\omega_x^2 + \omega_y^2 + \omega_z^2}$ |
| `+0x10` | `Quat*` | `current_quat` | Heap-allocated cumulative attitude quaternion (initialized to `(0, 0, 0, 1)`) |
| `+0x18` | `callback*` | `cb` | Registered callback table (`on_rotation_vector` at `cb + 0x08`) |

### 3.3 `android::Filter` (`sizeof = 384` bytes, `0x180`) — Site of `SAM-BUG-01`

| Offset | Type | Field Name | Description |
| :--- | :--- | :--- | :--- |
| `+0x000..0x12b` | `float[25][3]` | `samples` | Circular 3-axis sample buffer (`300` bytes cleared by `memset`, stride `12` bytes; overflows when constructed with `window_size = 50`) |
| `+0x12c` | `int32_t` | `window_size` | Moving-average capacity (`3`, `11`, or `50`) |
| `+0x130` | `int32_t` | `count` | Number of valid samples currently accumulated (`<= window_size`) |
| `+0x138` | `int32_t` | `write_index` | Next circular slot index: `(write_index + 1) % window_size` |
| `+0x140` | `float*` | `scratch_vec` | 12-byte heap allocation (`new float[3]`) |
| `+0x148..0x153` | `float[3]` | `average` | Latest 3-axis moving average (`avg_x`, `avg_y`, `avg_z`) |
| `+0x154..0x17b` | `pthread_mutex_t` | `mutex` | POSIX mutex guarding `movingAverage` updates |

---

## 4. `.rodata` & Immediate IEEE-754 Mathematical Constants

| Location | Hex Encoding | Type | Exact Value | Mathematical Role |
| :--- | :--- | :--- | :--- | :--- |
| `.rodata` `0x4c20` | `0x3f60624dd2f1a9fc` | `f64` | `0.002` | Maximum allowed 50-sample residual mean in `CalculateOffset::verAlgorithm` before triggering `initAlgorithm()` reset. |
| `.rodata` `0x4c28` | `0x3ee4f8b588e368f1` | `f64` | `1.0e-5` (`0.00001`) | Maximum 11-sample magnitude variance threshold in `IsDeviceNotInMotion_Gyro` and `SecGyroCalibration::process`. |
| `.rodata` `0x4c30` | `0x3fdfffd60e94ee39` | `f64` | `0.49999` | North-pole gimbal-lock singularity threshold in `getRotationMatrixFromVector` ($q_0 q_1 + q_2 q_3 > 0.49999 \|q\|^2$). |
| `.rodata` `0x4c38` | `0xbfdfffd60e94ee39` | `f64` | `-0.49999` | South-pole gimbal-lock singularity threshold in `getRotationMatrixFromVector` ($q_0 q_1 + q_2 q_3 < -0.49999 \|q\|^2$). |
| `.rodata` `0x4c40` | `0x42a00000` | `f32` | `80.0f` | Upper temperature bound (`0.0f <= temp < 80.0f`, else bin `0`) in `CalculateOffset::Insert`, `getmean`, and `GetOffset`. |
| `.rodata` `0x4c44` | `0xc2400000` | `f32` | `-48.0f` | ASCII digit `'0'` offset (`-48.0f`) used when parsing `/sys/class/sensors/gyro_sensor/temperature` in `getTemperatureThread`. |
| `.rodata` `0x4c48` | `0x4e6e6b28` | `f32` | `1.0e9f` (`1000000000.0f`) | Nanoseconds-to-seconds divisor in `RotationVector::ProcessAlgorithm` ($\Delta t = \text{f32}(t - t_0) / 10^9\text{f}$). |
| `.rodata` `0x4c4c` | `0x3ba3d70a` | `f32` | `0.005f` (`0.00499999989f`) | Minimum angular velocity norm threshold $\epsilon_\omega = 0.005\text{ rad/s}$ in `RotationVector::ProcessAlgorithm` for axis normalization. |
| Immediate `0x3eb4` | `imm8=0x60` (`fmov d1, #0.5`) | `f64` | `0.5` | Half-angle factor $\theta/2 = 0.5 \cdot \Delta t \cdot \|\omega\|_2$ in `RotationVector::ProcessAlgorithm`. |
| Immediate `0x3220` | `0x42652ee1` | `f32` | `57.29578f` | Radians-to-degrees constant ($180/\pi$) in `SecGyroCalibration`. |
| Immediate `0x3228` | `0x3c8efa35` | `f32` | `0.017453292f` | Degrees-to-radians constant ($\pi/180$) in `SecGyroCalibration`. |
