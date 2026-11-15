#![forbid(unsafe_code)]
//! Safe Rust reconstruction of `libgyroscope.camera.samsung.so`
//! (SHA-256: `d6a0bfb60a8352bb4029b4e39a99f280f8592693fa8e46734cc9547794f3ee97`,
//! `"GYRODRIFT :: Library Version - 1.20.3 Release Date - 06-March-19"`).
//! Replaces all `.bss` thread-hostile globals (`0x7008..0x7028`), raw pointer callbacks,
//! detached POSIX threads, and the 25-sample `Filter(50)` buffer overflow (`0x3b20`)
//! with 100% safe, pure, deterministic Rust structs (`#![forbid(unsafe_code)]`).

/// Maximum allowed 50-sample verification residual mean (`0x4c20`).
pub const VERIFICATION_RESIDUAL_LIMIT: f64 = 0.002;
/// Maximum 11-sample angular velocity magnitude variance for stationary detection (`0x4c28`).
pub const STATIONARY_VARIANCE_LIMIT: f64 = 1.0e-5;
/// North-pole gimbal-lock threshold (`0x4c30`).
pub const GIMBAL_LOCK_NORTH: f64 = 0.49999;
/// South-pole gimbal-lock threshold (`0x4c38`).
pub const GIMBAL_LOCK_SOUTH: f64 = -0.49999;
/// Temperature table upper bound (`80.0f`, `0x4c40`).
pub const MAX_TEMP_BIN: f32 = 80.0;
/// Nanoseconds-to-seconds divisor (`1.0e9f`, `0x4c48`).
pub const NS_PER_SEC_F32: f32 = 1.0e9;
/// Minimum angular velocity norm threshold (`0.005f`, `0x4c4c`) in `RotationVector::ProcessAlgorithm`.
pub const OMEGA_NORM_EPSILON: f32 = 0.005;

/// Attitude quaternion `(x, y, z, w)` matching `android::Quat` (`0x45b0..0x47f0`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Quat {
    /// Quaternion vector X component (`+0x00`).
    pub x: f32,
    /// Quaternion vector Y component (`+0x04`).
    pub y: f32,
    /// Quaternion vector Z component (`+0x08`).
    pub z: f32,
    /// Quaternion scalar W component (`+0x0c`).
    pub w: f32,
}

impl Default for Quat {
    fn default() -> Self {
        Self::identity()
    }
}

impl Quat {
    /// Returns the identity quaternion `(0.0, 0.0, 0.0, 1.0)` matching `Quat::loadIdentityQuat` (`0x4670`).
    #[inline(always)]
    pub const fn identity() -> Self {
        Self { x: 0.0, y: 0.0, z: 0.0, w: 1.0 }
    }

    /// Resets `self` to `(0.0, 0.0, 0.0, 1.0)` (`0x4670`).
    #[inline(always)]
    pub fn load_identity(&mut self) {
        *self = Self::identity();
    }

    /// Component-wise IEEE-754 equality check matching `Quat::equal` (`0x4730`).
    #[inline(always)]
    pub fn equal(&self, other: &Self) -> bool {
        self.x == other.x && self.y == other.y && self.z == other.z && self.w == other.w
    }

    /// Computes Hamilton product `lhs * self` (`q * d`) preserving the exact AArch64 `fmul`/`fadd`/`fsub`
    /// evaluation order of `android::Quat::multByQuat` (`0x47f0`).
    #[inline(always)]
    pub fn mult_by_quat(&self, lhs: &Self) -> Self {
        let (dx, dy, dz, dw) = (self.x, self.y, self.z, self.w);
        let (qx, qy, qz, qw) = (lhs.x, lhs.y, lhs.z, lhs.w);
        let ow = ((qw * dw - dx * qx) - dy * qy) - dz * qz;
        let ox = ((qx * dw + dx * qw) + qz * dy) - qy * dz;
        let oy = ((qy * dw + qw * dy) + qx * dz) - qz * dx;
        let oz = ((qz * dw + qw * dz) + qy * dx) - qx * dy;
        Self { x: ox, y: oy, z: oz, w: ow }
    }
}

/// Attitude quaternion integrator matching `android::RotationVector` (`0x3da0..0x3fa0`).
#[derive(Debug, Clone, Default)]
pub struct RotationVector {
    /// Previous gyroscope event timestamp in nanoseconds (`+0x00`).
    pub prev_timestamp_ns: i64,
    /// Latest angular velocity norm in rad/s (`+0x08`).
    pub omega_magnitude: f64,
    /// Cumulative attitude quaternion (`+0x10`).
    pub current_quat: Quat,
}

impl RotationVector {
    /// Creates a new `RotationVector` initialized to the identity quaternion.
    #[inline(always)]
    pub fn new() -> Self {
        Self { prev_timestamp_ns: 0, omega_magnitude: 0.0, current_quat: Quat::identity() }
    }

    /// Integrates one gyroscope sample (`sensor_type == 4`) over `timestamp_ns - prev_timestamp_ns`,
    /// matching `RotationVector::ProcessAlgorithm` (`0x3e10`).
    #[inline(always)]
    pub fn process_algorithm(&mut self, gyro: [f32; 3], sensor_type: i32, timestamp_ns: i64) {
        if sensor_type != 4 {
            return;
        }
        if self.prev_timestamp_ns == 0 {
            self.prev_timestamp_ns = timestamp_ns;
            return;
        }
        let dt_ns = timestamp_ns.wrapping_sub(self.prev_timestamp_ns);
        let dt_f32 = (dt_ns as f32) / NS_PER_SEC_F32;
        let dt_f64 = f64::from(dt_f32);

        let (gx, gy, gz) = (gyro[0], gyro[1], gyro[2]);
        let mag_sq = (gx * gx + gy * gy) + gz * gz;
        let mag = mag_sq.sqrt();
        self.omega_magnitude = f64::from(mag);

        let (nx_div, ny_div, nz_div) = (gx / mag, gy / mag, gz / mag);
        let (nx, ny, nz) = if mag <= OMEGA_NORM_EPSILON { (gx, gy, gz) } else { (nx_div, ny_div, nz_div) };

        let half_theta = (dt_f64 * f64::from(mag)) * 0.5;
        let (s, c) = (half_theta.sin(), half_theta.cos());
        let delta_q = Quat {
            x: (f64::from(nx) * s) as f32,
            y: (f64::from(ny) * s) as f32,
            z: (f64::from(nz) * s) as f32,
            w: -(c as f32),
        };
        self.current_quat = delta_q.mult_by_quat(&self.current_quat);
        self.prev_timestamp_ns = timestamp_ns;
    }
}

/// Converts a quaternion `q = [q0, q1, q2, q3]` (`x, y, z, w`) into a 4×4 row-major
/// rotation matrix (with negated-`q3` off-diagonal conventions) and 3-axis Euler angles
/// (with `±0.49999` gimbal-lock guards), matching `GyroscopeLib::getRotationMatrixFromVector` (`0x43d0`).
#[inline(always)]
pub fn get_rotation_matrix_from_vector(q: [f32; 4]) -> ([f32; 16], [f32; 3]) {
    let (q0, q1, q2, q3) = (q[0], q[1], q[2], q[3]);
    let (two_q0, two_q1) = (q0 + q0, q1 + q1);
    let (two_q1_sq, two_q0_sq) = (q1 * two_q1, q0 * two_q0);
    let two_q2 = q2 + q2;
    let two_q2_sq = q2 * two_q2;
    let neg_q3 = -q3;

    let (two_q2_nq3, two_q1_nq3) = (two_q2 * neg_q3, two_q1 * neg_q3);
    let (two_q1_q2, two_q0_nq3) = (two_q1 * q2, two_q0 * neg_q3);
    let (q1_sq, q2_sq, q0_sq, q3_sq) = (q1 * q1, q2 * q2, q0 * q0, q3 * q3);
    let (two_q0_q1, two_q0_q2) = (two_q0 * q1, two_q0 * q2);
    let test = q0 * q1 + q2 * q3;

    let r_mat = [
        (1.0 - two_q1_sq) - two_q2_sq, two_q0_q1 - two_q2_nq3, two_q0_q2 + two_q1_nq3, 0.0,
        two_q0_q1 + two_q2_nq3, (1.0 - two_q0_sq) - two_q2_sq, two_q1_q2 - two_q0_nq3, 0.0,
        two_q0_q2 - two_q1_nq3, two_q1_q2 + two_q0_nq3, (1.0 - two_q0_sq) - two_q1_sq, 0.0,
        0.0, 0.0, 0.0, 1.0,
    ];

    let sq_norm = q3_sq + (q0_sq + (q1_sq + q2_sq));
    let (d_test, d_sq_norm) = (f64::from(test), f64::from(sq_norm));
    let euler = if d_test > d_sq_norm * GIMBAL_LOCK_NORTH {
        let y = q0.atan2(q3);
        [1.0, 0.0, y + y]
    } else if d_test < d_sq_norm * GIMBAL_LOCK_SOUTH {
        let y = q0.atan2(q3);
        [-1.0, 0.0, y * -2.0]
    } else {
        let e2 = (two_q1 * q3 - two_q0_q2).atan2(((q0_sq - q1_sq) - q2_sq) + q3_sq);
        let e0 = ((test + test) / sq_norm).asin();
        let e1 = (two_q0 * q3 - two_q1_q2).atan2(((q1_sq - q0_sq) - q2_sq) + q3_sq);
        [e0, e1, e2]
    };
    (r_mat, euler)
}

/// Fixed-capacity 3-axis moving average and variance filter matching `android::Filter` (`0x3a90..0x3d98`).
/// Fixes the `Filter(50)` 25-sample buffer overflow (`SAM-BUG-01`) in `libgyroscope.camera.samsung.so`.
#[derive(Debug, Clone)]
pub struct Filter<const N: usize> {
    /// Circular buffer of 3-axis samples (`[[x, y, z]; N]`).
    pub samples: [[f32; 3]; N],
    /// Number of valid accumulated samples (`<= N`).
    pub count: usize,
    /// Next circular write slot (`0..N`).
    pub write_index: usize,
    /// Latest 3-axis moving average (`[avg_x, avg_y, avg_z]`).
    pub average: [f32; 3],
}

impl<const N: usize> Default for Filter<N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const N: usize> Filter<N> {
    /// Creates a zero-initialized `Filter<N>`.
    #[inline(always)]
    pub const fn new() -> Self {
        Self { samples: [[0.0; 3]; N], count: 0, write_index: 0, average: [0.0; 3] }
    }

    /// Resets `count`, `write_index`, `average`, and sample buffer (`0x3ca0`).
    #[inline(always)]
    pub fn reset(&mut self) {
        *self = Self::new();
    }

    /// Appends `(x, y, z)` and computes the updated moving average (`0x3b20`).
    #[inline(always)]
    pub fn moving_average(&mut self, x: f32, y: f32, z: f32) -> [f32; 3] {
        if N == 0 {
            return [0.0; 3];
        }
        let idx = self.write_index % N;
        self.samples[idx] = [x, y, z];
        self.write_index = (idx + 1) % N;
        if self.count < N {
            self.count += 1;
        }
        let (mut sx, mut sy, mut sz) = (0.0_f32, 0.0_f32, 0.0_f32);
        for sample in self.samples.iter().take(self.count) {
            sx += sample[0];
            sy += sample[1];
            sz += sample[2];
        }
        let fn_cnt = self.count as f32;
        let out = [sx / fn_cnt, sy / fn_cnt, sz / fn_cnt];
        self.average = out;
        out
    }

    /// Computes the 3-axis sample variance around `self.average` (`0x3cf0`).
    #[inline(always)]
    pub fn get_variance(&self) -> [f32; 3] {
        if self.count == 0 {
            return [0.0; 3];
        }
        let (ax, ay, az) = (self.average[0], self.average[1], self.average[2]);
        let (mut vx, mut vy, mut vz) = (0.0_f32, 0.0_f32, 0.0_f32);
        for sample in self.samples.iter().take(self.count) {
            let dx = sample[0] - ax;
            vx += dx * dx;
            let dy = sample[1] - ay;
            vy += dy * dy;
            let dz = sample[2] - az;
            vz += dz * dz;
        }
        let fn_cnt = self.count as f32;
        [vx / fn_cnt, vy / fn_cnt, vz / fn_cnt]
    }
}

/// Least-squares linear regression helper matching `CalculateOffset::LinearFitting` (`0x2f70`).
#[inline(always)]
pub fn calculate_offset_linear_fitting(
    samples_x: &[f32],
    samples_y: &[f32],
    sum_x: f32,
    sum_y: f32,
) -> (f32, f32) {
    let n = samples_x.len().min(samples_y.len());
    if n == 0 {
        return (0.0, 0.0);
    }
    let fn_n = n as f32;
    let (mean_y, mean_x) = (sum_y / fn_n, sum_x / fn_n);
    let (mut cov_xy, mut var_x) = (0.0_f32, 0.0_f32);
    for i in 0..n {
        let dx = samples_x[i] - mean_x;
        var_x += dx * dx;
        let dy = samples_y[i] - mean_y;
        cov_xy += dx * dy;
    }
    if var_x == 0.0 {
        return (0.0, mean_y);
    }
    let slope = cov_xy / var_x;
    (slope, mean_y - mean_x * slope)
}

/// Evaluates stationary state from an 11-sample angular velocity magnitude filter,
/// matching `SecGyroCalibration::IsDeviceNotInMotion_Gyro` (`0x34f0`).
#[inline(always)]
pub fn is_device_not_in_motion_gyro(
    filter_mag_11: &mut Filter<11>,
    stationary_counter: &mut i32,
    stationary_threshold: i32,
    gyro: [f32; 3],
) -> bool {
    let (gx, gy, gz) = (gyro[0], gyro[1], gyro[2]);
    let mag = ((gx * gx + gy * gy) + gz * gz).sqrt();
    let _ = filter_mag_11.moving_average(mag, 0.0, 0.0);
    let var_x = f64::from(filter_mag_11.get_variance()[0]);

    let mut is_stationary = false;
    if var_x < STATIONARY_VARIANCE_LIMIT {
        let prev = *stationary_counter;
        *stationary_counter = prev.wrapping_add(1);
        if prev >= stationary_threshold {
            is_stationary = true;
        }
    }
    if var_x > STATIONARY_VARIANCE_LIMIT {
        *stationary_counter = 0;
    }
    is_stationary
}

/// FFI parity verification entry point for `Quat::multByQuat` (`0x47f0`).
#[inline(always)]
#[allow(clippy::too_many_arguments)]
pub fn quat_mult_w(dx: f32, dy: f32, dz: f32, dw: f32, qx: f32, qy: f32, qz: f32, qw: f32) -> f32 {
    if !(dx.is_finite() && dy.is_finite() && dz.is_finite() && dw.is_finite()
        && qx.is_finite() && qy.is_finite() && qz.is_finite() && qw.is_finite())
    {
        return 0.0;
    }
    let d = Quat { x: dx, y: dy, z: dz, w: dw };
    let q = Quat { x: qx, y: qy, z: qz, w: qw };
    let out = d.mult_by_quat(&q);
    if out.w.is_finite() { out.w } else { 0.0 }
}

/// FFI parity verification entry point for `RotationVector::ProcessAlgorithm` (`0x3e10`).
#[inline(always)]
#[allow(clippy::too_many_arguments)]
pub fn rotation_vector_step_w(gx: f32, gy: f32, gz: f32, dt_ns: i64, qx: f32, qy: f32, qz: f32, qw: f32) -> f32 {
    if !(gx.is_finite() && gy.is_finite() && gz.is_finite()
        && qx.is_finite() && qy.is_finite() && qz.is_finite() && qw.is_finite())
    {
        return 0.0;
    }
    let mut rv = RotationVector {
        prev_timestamp_ns: 1,
        omega_magnitude: 0.0,
        current_quat: Quat { x: qx, y: qy, z: qz, w: qw },
    };
    rv.process_algorithm([gx, gy, gz], 4, 1_i64.wrapping_add(dt_ns));
    if rv.current_quat.w.is_finite() { rv.current_quat.w } else { 0.0 }
}

/// Deterministic single-slice FFI parity verification entry point exercising
/// `Filter`, `is_device_not_in_motion_gyro`, `calculate_offset_linear_fitting`,
/// and `get_rotation_matrix_from_vector`.
#[inline(always)]
pub fn gyro_calibration_pipeline(data: &[u8]) -> f32 {
    let mut filter_mag = Filter::<11>::new();
    let mut stationary_counter = 0_i32;
    let mut stationary_hits = 0_i32;
    let mut gx_hist = [0.0_f32; 4];
    let mut last_gyro = [0.0_f32; 3];

    for i in 0..16usize {
        let b0 = if data.is_empty() { 0 } else { data[(i * 3) % data.len()] };
        let b1 = if data.is_empty() { 0 } else { data[(i * 3 + 1) % data.len()] };
        let b2 = if data.is_empty() { 0 } else { data[(i * 3 + 2) % data.len()] };
        let gx = ((i32::from(b0) - 128) as f32) * 0.0001_f32;
        let gy = ((i32::from(b1) - 128) as f32) * 0.0001_f32;
        let gz = ((i32::from(b2) - 128) as f32) * 0.0001_f32;
        if i < 4 {
            gx_hist[i] = gx * 1000.0_f32;
        }
        last_gyro = [gx, gy, gz];
        if is_device_not_in_motion_gyro(&mut filter_mag, &mut stationary_counter, 10, last_gyro) {
            stationary_hits += 1;
        }
    }

    let temps = [20.0_f32, 25.0_f32, 30.0_f32, 35.0_f32];
    let sum_x = temps[0] + temps[1] + temps[2] + temps[3];
    let sum_y = gx_hist[0] + gx_hist[1] + gx_hist[2] + gx_hist[3];
    let (slope, intercept) = calculate_offset_linear_fitting(&temps, &gx_hist, sum_x, sum_y);
    let q = [last_gyro[0] * 10.0_f32, last_gyro[1] * 10.0_f32, last_gyro[2] * 10.0_f32, 1.0_f32];
    let (r_mat, euler) = get_rotation_matrix_from_vector(q);

    ((r_mat[0] + r_mat[5] + r_mat[10]) + (euler[0] + euler[1] + euler[2]))
        + (slope + intercept)
        + (stationary_hits as f32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quat_filter_and_pipeline() {
        let id = Quat::identity();
        let q = Quat { x: 0.1, y: -0.2, z: 0.3, w: 0.9 };
        assert!(id.mult_by_quat(&q).equal(&q));
        let mut f = Filter::<50>::new();
        for i in 0..60 {
            assert!(f.moving_average(i as f32, 1.0, -1.0)[0].is_finite());
        }
        assert_eq!(f.count, 50);
        assert!(gyro_calibration_pipeline(&[128, 128, 128]).is_finite());
    }
}
