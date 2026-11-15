#![forbid(unsafe_code)]
//! Standalone regression test suite verifying recorded C oracle test vectors
//! (`0 ULP` bit-exact equivalence) and `SAM-BUG-01` `Filter<50>` bounds safety
//! against the reconstructed Safe Rust crate `s9_camera_gyroscope`.

use s9_camera_gyroscope::{
    gyro_calibration_pipeline, quat_mult_w, rotation_vector_step_w, Filter,
};

#[test]
fn verified_c_oracle_regression_vectors() {
    // Vector #1: inputs=(empty) -> -4.7821293 (0xc0990734)
    {
        let actual = gyro_calibration_pipeline(&[]);
        let expected = f32::from_bits(0xc0990734u32);
        assert_eq!(
            actual.to_bits(),
            expected.to_bits(),
            "Vector #1 mismatch: actual={actual:?}, expected={expected:?}"
        );
    }

    // Vector #2: inputs=(0x00) -> -4.7821293 (0xc0990734)
    {
        let actual = gyro_calibration_pipeline(&[0x00u8]);
        let expected = f32::from_bits(0xc0990734u32);
        assert_eq!(
            actual.to_bits(),
            expected.to_bits(),
            "Vector #2 mismatch: actual={actual:?}, expected={expected:?}"
        );
    }

    // Vector #3: inputs=(0xff) -> 22.226357 (0x41b1cf94)
    {
        let actual = gyro_calibration_pipeline(&[0xffu8]);
        let expected = f32::from_bits(0x41b1cf94u32);
        assert_eq!(
            actual.to_bits(),
            expected.to_bits(),
            "Vector #3 mismatch: actual={actual:?}, expected={expected:?}"
        );
    }

    // Vector #4: inputs=("ARGON_ZEROG_PARITY_V1") -> 0.92722034 (0x3f6d5e50)
    {
        let actual = gyro_calibration_pipeline(b"ARGON_ZEROG_PARITY_V1");
        let expected = f32::from_bits(0x3f6d5e50u32);
        assert_eq!(
            actual.to_bits(),
            expected.to_bits(),
            "Vector #4 mismatch: actual={actual:?}, expected={expected:?}"
        );
    }

    // Vector #5: inputs=(0x00..=0x0f) -> -5.7947598 (0xc0b96eac)
    {
        let input: [u8; 16] = [
            0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d,
            0x0e, 0x0f,
        ];
        let actual = gyro_calibration_pipeline(&input);
        let expected = f32::from_bits(0xc0b96eacu32);
        assert_eq!(
            actual.to_bits(),
            expected.to_bits(),
            "Vector #5 mismatch: actual={actual:?}, expected={expected:?}"
        );
    }

    // Vector #6: inputs=(64x 0xa5) -> 12.902549 (0x414e70d7)
    {
        let input = [0xa5u8; 64];
        let actual = gyro_calibration_pipeline(&input);
        let expected = f32::from_bits(0x414e70d7u32);
        assert_eq!(
            actual.to_bits(),
            expected.to_bits(),
            "Vector #6 mismatch: actual={actual:?}, expected={expected:?}"
        );
    }
}

#[test]
fn verified_quat_and_rotation_vector_oracles() {
    // Identity quaternion composition must preserve W component bit-for-bit
    let w = quat_mult_w(0.0, 0.0, 0.0, 1.0, 0.25, -0.5, 0.125, 0.75);
    assert_eq!(w.to_bits(), 0.75_f32.to_bits());

    // Stationary rotation step over 2.4ms (416Hz period = 2_403_846 ns)
    let step_w = rotation_vector_step_w(0.01, -0.02, 0.005, 2_403_846, 0.0, 0.0, 0.0, 1.0);
    assert!(step_w.is_finite());
}

#[test]
fn sam_bug_01_filter_50_bounds_safety() {
    // In C/C++ (libgyroscope.camera.samsung.so at 0x3a90 / 0x3b20), Filter(50) writes
    // samples 25..49 past the 25-element inline array into window_size, count, write_index,
    // scratch_vec pointer, and pthread_mutex_t.
    // In Safe Rust, Filter<50> allocates [[f32; 3]; 50] at compile time and wraps safely.
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
