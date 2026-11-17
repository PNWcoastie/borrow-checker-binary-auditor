#![forbid(unsafe_code)]
//! Standalone regression test suite verifying recorded C oracle test vectors
//! (`0 ULP` bit-exact equivalence), all `30,155` deterministic SplitMix64
//! (`seed = 1337420`) + boundary FFI parity vectors against standalone AArch64
//! instruction-level reference oracles with pinned 64-bit FNV-1a output digests,
//! and `SAM-BUG-01` `Filter<50>` bounds safety against `s9_camera_gyroscope`.

use s9_camera_gyroscope::{
    gyro_calibration_pipeline, quat_mult_w, rotation_vector_step_w, Filter,
};

struct SplitMix64 {
    state: u64,
}

impl SplitMix64 {
    const fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    #[inline(always)]
    fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9e3779b97f4a7c15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
        z ^ (z >> 31)
    }

    #[inline(always)]
    fn next_f32(&mut self) -> f32 {
        let bits = self.next_u64() as u32;
        let cand = f32::from_bits(bits);
        if cand.is_finite() && cand.abs() <= 1.0e4 {
            cand
        } else {
            ((bits as i32) as f32) * (1.0 / 2_147_483_648.0) * 100.0
        }
    }
}

struct Fnv1a64 {
    hash: u64,
}

impl Fnv1a64 {
    const OFFSET_BASIS: u64 = 0xcbf29ce484222325;
    const PRIME: u64 = 0x100000001b3;

    const fn new() -> Self {
        Self { hash: Self::OFFSET_BASIS }
    }

    #[inline(always)]
    fn write_f32_bits(&mut self, val: f32) {
        for b in val.to_bits().to_le_bytes() {
            self.hash ^= u64::from(b);
            self.hash = self.hash.wrapping_mul(Self::PRIME);
        }
    }

    const fn finish(&self) -> u64 {
        self.hash
    }
}

#[inline(always)]
#[allow(clippy::too_many_arguments)]
fn c_oracle_quat_mult_w(dx: f32, dy: f32, dz: f32, dw: f32, qx: f32, qy: f32, qz: f32, qw: f32) -> f32 {
    if !(dx.is_finite() && dy.is_finite() && dz.is_finite() && dw.is_finite()
        && qx.is_finite() && qy.is_finite() && qz.is_finite() && qw.is_finite())
    {
        return 0.0;
    }
    // AArch64 _ZN7android4Quat10multByQuatEPS0_S1_ (0x47f0)
    let ow = ((qw * dw - dx * qx) - dy * qy) - dz * qz;
    if ow.is_finite() { ow } else { 0.0 }
}

#[inline(always)]
#[allow(clippy::too_many_arguments)]
fn c_oracle_rotation_vector_step_w(
    gx: f32, gy: f32, gz: f32, dt_ns: i64, qx: f32, qy: f32, qz: f32, qw: f32,
) -> f32 {
    if !(gx.is_finite() && gy.is_finite() && gz.is_finite()
        && qx.is_finite() && qy.is_finite() && qz.is_finite() && qw.is_finite())
    {
        return 0.0;
    }
    // AArch64 _ZN7android14RotationVector16ProcessAlgorithmENS_3vecIfLm3EEEix (0x3e10)
    let dt_diff = (1_i64.wrapping_add(dt_ns)).wrapping_sub(1_i64);
    let dt_f32 = (dt_diff as f32) / f32::from_bits(0x4e6e6b28); // 1.0e9f at 0x4c48
    let dt_f64 = f64::from(dt_f32);
    let mag = ((gx * gx + gy * gy) + gz * gz).sqrt();
    let eps = f32::from_bits(0x3ba3d70a); // 0.005f at 0x4c4c
    let (nx, ny, nz) = if mag <= eps { (gx, gy, gz) } else { (gx / mag, gy / mag, gz / mag) };
    let half_theta = (dt_f64 * f64::from(mag)) * 0.5;
    let (s, c) = (half_theta.sin(), half_theta.cos());
    let (dx, dy, dz, dw) = (
        (f64::from(nx) * s) as f32,
        (f64::from(ny) * s) as f32,
        (f64::from(nz) * s) as f32,
        -(c as f32),
    );
    let ow = ((qw * dw - dx * qx) - dy * qy) - dz * qz;
    if ow.is_finite() { ow } else { 0.0 }
}

#[inline(always)]
fn c_oracle_gyro_calibration_pipeline(data: &[u8]) -> f32 {
    let mut samples = [0.0_f32; 11];
    let (mut count, mut write_idx) = (0_usize, 0_usize);
    let (mut stationary_counter, mut stationary_hits) = (0_i32, 0_i32);
    let mut gx_hist = [0.0_f32; 4];
    let mut last_gyro = [0.0_f32; 3];
    let var_limit = f64::from_bits(0x3ee4f8b588e368f1); // 1.0e-5 at 0x4c28

    for i in 0..16_usize {
        let (b0, b1, b2) = if data.is_empty() {
            (0, 0, 0)
        } else {
            (data[(i * 3) % data.len()], data[(i * 3 + 1) % data.len()], data[(i * 3 + 2) % data.len()])
        };
        let gx = ((i32::from(b0) - 128) as f32) * 0.0001_f32;
        let gy = ((i32::from(b1) - 128) as f32) * 0.0001_f32;
        let gz = ((i32::from(b2) - 128) as f32) * 0.0001_f32;
        if i < 4 {
            gx_hist[i] = gx * 1000.0_f32;
        }
        last_gyro = [gx, gy, gz];

        let mag = ((gx * gx + gy * gy) + gz * gz).sqrt();
        let slot = write_idx % 11;
        samples[slot] = mag;
        write_idx = (slot + 1) % 11;
        if count < 11 {
            count += 1;
        }
        let mut sx = 0.0_f32;
        for &s in samples.iter().take(count) {
            sx += s;
        }
        let avg_x = sx / (count as f32);
        let mut vx = 0.0_f32;
        for &s in samples.iter().take(count) {
            let dx = s - avg_x;
            vx += dx * dx;
        }
        let var_x = f64::from(vx / (count as f32));
        if var_x < var_limit {
            let prev = stationary_counter;
            stationary_counter = prev.wrapping_add(1);
            if prev >= 10 {
                stationary_hits += 1;
            }
        }
        if var_x > var_limit {
            stationary_counter = 0;
        }
    }

    let temps = [20.0_f32, 25.0_f32, 30.0_f32, 35.0_f32];
    let mean_x = (temps[0] + temps[1] + temps[2] + temps[3]) / 4.0_f32;
    let mean_y = (gx_hist[0] + gx_hist[1] + gx_hist[2] + gx_hist[3]) / 4.0_f32;
    let (mut cov_xy, mut var_x) = (0.0_f32, 0.0_f32);
    for i in 0..4 {
        let dx = temps[i] - mean_x;
        var_x += dx * dx;
        cov_xy += dx * (gx_hist[i] - mean_y);
    }
    let (slope, intercept) = if var_x == 0.0 {
        (0.0, mean_y)
    } else {
        let s = cov_xy / var_x;
        (s, mean_y - mean_x * s)
    };

    let (q0, q1, q2, q3) = (last_gyro[0] * 10.0_f32, last_gyro[1] * 10.0_f32, last_gyro[2] * 10.0_f32, 1.0_f32);
    let (two_q0, two_q1, two_q2) = (q0 + q0, q1 + q1, q2 + q2);
    let (two_q0_sq, two_q1_sq, two_q2_sq) = (q0 * two_q0, q1 * two_q1, q2 * two_q2);
    let (q1_sq, q2_sq, q0_sq, q3_sq) = (q1 * q1, q2 * q2, q0 * q0, q3 * q3);
    let (two_q1_q2, two_q0_q2) = (two_q1 * q2, two_q0 * q2);
    let test = q0 * q1 + q2 * q3;
    let (r0, r5, r10) = ((1.0 - two_q1_sq) - two_q2_sq, (1.0 - two_q0_sq) - two_q2_sq, (1.0 - two_q0_sq) - two_q1_sq);

    let sq_norm = q3_sq + (q0_sq + (q1_sq + q2_sq));
    let (d_test, d_sq_norm) = (f64::from(test), f64::from(sq_norm));
    let [e0, e1, e2] = if d_test > d_sq_norm * f64::from_bits(0x3fdfffd60e94ee39) {
        let y = q0.atan2(q3);
        [1.0, 0.0, y + y]
    } else if d_test < d_sq_norm * f64::from_bits(0xbfdfffd60e94ee39) {
        let y = q0.atan2(q3);
        [-1.0, 0.0, y * -2.0]
    } else {
        [
            ((test + test) / sq_norm).asin(),
            (two_q0 * q3 - two_q1_q2).atan2(((q1_sq - q0_sq) - q2_sq) + q3_sq),
            (two_q1 * q3 - two_q0_q2).atan2(((q0_sq - q1_sq) - q2_sq) + q3_sq),
        ]
    };

    ((r0 + r5 + r10) + (e0 + e1 + e2)) + (slope + intercept) + (stationary_hits as f32)
}

#[test]
fn verified_c_oracle_regression_vectors() {
    let cases: &[(&[u8], u32)] = &[
        (&[], 0xc0990734),
        (&[0x00], 0xc0990734),
        (&[0xff], 0x41b1cf94),
        (b"ARGON_ZEROG_PARITY_V1", 0x3f6d5e50),
        (
            &[0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e, 0x0f],
            0xc0b96eac,
        ),
        (&[0xa5; 64], 0x414e70d7),
    ];
    for (idx, &(input, expected_bits)) in cases.iter().enumerate() {
        let actual = gyro_calibration_pipeline(input);
        assert_eq!(actual.to_bits(), expected_bits, "Vector #{} mismatch", idx + 1);
    }
}

#[test]
fn verified_quat_and_rotation_vector_oracles() {
    let w = quat_mult_w(0.0, 0.0, 0.0, 1.0, 0.25, -0.5, 0.125, 0.75);
    assert_eq!(w.to_bits(), 0.75_f32.to_bits());

    let step_w = rotation_vector_step_w(0.01, -0.02, 0.005, 2_403_846, 0.0, 0.0, 0.0, 1.0);
    assert!(step_w.is_finite());
}

#[test]
fn sam_bug_01_filter_50_bounds_safety() {
    let mut filter = Filter::<50>::new();
    for sample_idx in 0..100 {
        let avg = filter.moving_average(sample_idx as f32, 2.0, -2.0);
        assert!(avg[0].is_finite());
        assert!((avg[1] - 2.0).abs() < 1e-5);
        assert!((avg[2] + 2.0).abs() < 1e-5);
        assert!(filter.count <= 50);
        assert!(filter.write_index < 50);
    }
}

#[test]
fn splitmix64_30155_ffi_parity_vectors_and_fnv1a_digests() {
    const SEED: u64 = 1337420;
    const F32_EDGES: [f32; 16] = [
        0.0, -0.0, 1.0, -1.0, 0.5, -0.5, 1.0e-6, -1.0e-6,
        0.005, 0.0049999, 0.0050001, 1.0e3, -1.0e3, f32::NAN, f32::INFINITY, f32::NEG_INFINITY,
    ];
    const DT_EDGES: [i64; 8] = [0, 1, -1, 2_403_846, 1_000_000_000, -2_403_846, i64::MAX, i64::MIN];

    // 1. quat_mult_w: 64 IEEE-754 boundary vectors + 10,000 SplitMix64 vectors = 10,064
    let mut q_rng = SplitMix64::new(SEED);
    let mut q_hash = Fnv1a64::new();
    let mut q_count = 0_usize;
    for i in 0..64_usize {
        let a = [
            F32_EDGES[i % 16], F32_EDGES[(i + 1) % 16], F32_EDGES[(i + 2) % 16], F32_EDGES[(i + 3) % 16],
            F32_EDGES[(i / 4) % 16], F32_EDGES[(i / 4 + 5) % 16], F32_EDGES[(i / 4 + 9) % 16], F32_EDGES[(i / 4 + 13) % 16],
        ];
        let actual = quat_mult_w(a[0], a[1], a[2], a[3], a[4], a[5], a[6], a[7]);
        let oracle = c_oracle_quat_mult_w(a[0], a[1], a[2], a[3], a[4], a[5], a[6], a[7]);
        assert_eq!(actual.to_bits(), oracle.to_bits());
        q_hash.write_f32_bits(actual);
        q_count += 1;
    }
    for _ in 0..10_000_usize {
        let a = [
            q_rng.next_f32(), q_rng.next_f32(), q_rng.next_f32(), q_rng.next_f32(),
            q_rng.next_f32(), q_rng.next_f32(), q_rng.next_f32(), q_rng.next_f32(),
        ];
        let actual = quat_mult_w(a[0], a[1], a[2], a[3], a[4], a[5], a[6], a[7]);
        let oracle = c_oracle_quat_mult_w(a[0], a[1], a[2], a[3], a[4], a[5], a[6], a[7]);
        assert_eq!(actual.to_bits(), oracle.to_bits());
        q_hash.write_f32_bits(actual);
        q_count += 1;
    }
    assert_eq!(q_count, 10_064);
    assert_eq!(q_hash.finish(), 0x71aa_7ddf_5dae_91ca);

    // 2. rotation_vector_step_w: 64 IEEE-754/dt boundary vectors + 10,000 SplitMix64 vectors = 10,064
    let mut rv_rng = SplitMix64::new(SEED);
    let mut rv_hash = Fnv1a64::new();
    let mut rv_count = 0_usize;
    for i in 0..64_usize {
        let (gx, gy, gz) = (F32_EDGES[i % 16], F32_EDGES[(i + 3) % 16], F32_EDGES[(i + 7) % 16]);
        let dt_ns = DT_EDGES[i % 8];
        let (qx, qy, qz, qw) = (
            F32_EDGES[(i / 2) % 16], F32_EDGES[(i / 2 + 2) % 16], F32_EDGES[(i / 2 + 4) % 16], F32_EDGES[(i / 2 + 6) % 16],
        );
        let actual = rotation_vector_step_w(gx, gy, gz, dt_ns, qx, qy, qz, qw);
        let oracle = c_oracle_rotation_vector_step_w(gx, gy, gz, dt_ns, qx, qy, qz, qw);
        assert_eq!(actual.to_bits(), oracle.to_bits());
        rv_hash.write_f32_bits(actual);
        rv_count += 1;
    }
    for _ in 0..10_000_usize {
        let (gx, gy, gz) = (rv_rng.next_f32(), rv_rng.next_f32(), rv_rng.next_f32());
        let dt_ns = (rv_rng.next_u64() % 50_000_000) as i64;
        let (qx, qy, qz, qw) = (rv_rng.next_f32(), rv_rng.next_f32(), rv_rng.next_f32(), rv_rng.next_f32());
        let actual = rotation_vector_step_w(gx, gy, gz, dt_ns, qx, qy, qz, qw);
        let oracle = c_oracle_rotation_vector_step_w(gx, gy, gz, dt_ns, qx, qy, qz, qw);
        assert_eq!(actual.to_bits(), oracle.to_bits());
        rv_hash.write_f32_bits(actual);
        rv_count += 1;
    }
    assert_eq!(rv_count, 10_064);
    assert_eq!(rv_hash.finish(), 0x8086_eeb7_6231_94de);

    // 3. gyro_calibration_pipeline: 27 boundary vectors + 10,000 SplitMix64 vectors = 10,027
    let mut gp_rng = SplitMix64::new(SEED);
    let mut gp_hash = Fnv1a64::new();
    let mut gp_count = 0_usize;
    let mut buf = [0_u8; 64];
    for b_idx in 0..27_usize {
        let slice: &[u8] = match b_idx {
            0 => &[],
            1 => &[0x00],
            2 => &[0xff],
            3 => b"ARGON_ZEROG_PARITY_V1",
            4 => &[0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e, 0x0f],
            5 => &[0xa5; 64],
            6 => &[0x80; 64],
            7 => &[0x00; 64],
            8 => &[0xff; 64],
            _ => {
                let len = b_idx - 7;
                for (k, slot) in buf.iter_mut().take(len).enumerate() {
                    *slot = if (k + b_idx) % 2 == 0 { 0x55 } else { 0xaa };
                }
                &buf[..len]
            }
        };
        let actual = gyro_calibration_pipeline(slice);
        let oracle = c_oracle_gyro_calibration_pipeline(slice);
        assert_eq!(actual.to_bits(), oracle.to_bits());
        gp_hash.write_f32_bits(actual);
        gp_count += 1;
    }
    for _ in 0..10_000_usize {
        let len = ((gp_rng.next_u64() as usize) % 64) + 1;
        for chunk in buf[..len].chunks_mut(8) {
            let bytes = gp_rng.next_u64().to_le_bytes();
            chunk.copy_from_slice(&bytes[..chunk.len()]);
        }
        let actual = gyro_calibration_pipeline(&buf[..len]);
        let oracle = c_oracle_gyro_calibration_pipeline(&buf[..len]);
        assert_eq!(actual.to_bits(), oracle.to_bits());
        gp_hash.write_f32_bits(actual);
        gp_count += 1;
    }
    assert_eq!(gp_count, 10_027);
    assert_eq!(gp_hash.finish(), 0xa417_22ae_ad52_b64d);
    assert_eq!(q_count + rv_count + gp_count, 30_155);
}
