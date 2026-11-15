## Summary

<!-- Briefly describe the purpose of this Pull Request and reference the tracking issue (e.g., Closes #1). -->

## Verification & Safety Checklist

- [ ] **Zero `unsafe` Code:** Crate root retains `#![forbid(unsafe_code)]` with zero unsafe blocks or FFI workarounds in library code.
- [ ] **Bit-Exact (`0 ULP`) Parity Preserved:** `cargo test` passes all unit and C-oracle regression vectors without relaxing IEEE-754 floating-point tolerances.
- [ ] **Zero Warnings:** `cargo clippy --all-targets -- -D warnings` and `cargo doc --no-deps` complete with `0` warnings.
- [ ] **Clean-Room & Sanitization Boundary:** No proprietary OEM `.so` binaries, firmware images, unredacted exploit payloads, `.env*` files, or local machine paths are included.
