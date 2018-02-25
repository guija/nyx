pub use super::RK;

pub struct Dormand54 {}

/// Dormand54 is a [Dormand-Prince integrator](https://en.wikipedia.org/wiki/Dormand%E2%80%93Prince_method).
impl RK for Dormand54 {
    fn order() -> usize {
        5 as usize
    }

    fn a_coeffs() -> &'static [f64] {
        &[
            1.0 / 5.0,
            3.0 / 40.0,
            9.0 / 40.0,
            44.0 / 45.0,
            -56.0 / 15.0,
            32.0 / 9.0,
            19372.0 / 6561.0,
            -25360.0 / 2187.0,
            64448.0 / 6561.0,
            -212.0 / 729.0,
            9017.0 / 3168.0,
            -355.0 / 33.0,
            46732.0 / 5247.0,
            49.0 / 176.0,
            -5103.0 / 18656.0,
        ]
    }
    fn b_coeffs() -> &'static [f64] {
        &[
            35.0 / 384.0,
            0.0,
            500.0 / 1113.0,
            125.0 / 192.0,
            -2187.0 / 6784.0,
            11.0 / 84.0,
            5179.0 / 57600.0,
            0.0,
            7571.0 / 16695.0,
            393.0 / 640.0,
            -92097.0 / 339200.0,
            187.0 / 2100.0,
            1.0 / 40.0,
        ]
    }
}
