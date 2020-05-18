extern crate csv;
extern crate hifitime;
extern crate nalgebra as na;
extern crate nyx_space as nyx;

use self::hifitime::Epoch;
use self::nyx::celestia::State;
use self::nyx::od::ui::{Estimate, Filter, FilterError, KfEstimate, KF};

mod measurements;
mod multi_body;
mod spacecraft;
mod srif;
mod two_body;

use self::na::{Matrix2, Matrix2x6, Matrix3, Matrix6, Vector2};
use std::f64::EPSILON;

macro_rules! f64_nil {
    ($x:expr, $msg:expr) => {
        assert!($x.abs() < EPSILON, $msg)
    };
}

#[test]
fn empty_estimate() {
    let empty = KfEstimate::zeros(State::zeros());
    f64_nil!(
        empty.state_deviation.norm(),
        "expected state norm to be nil"
    );
    f64_nil!(empty.covar.norm(), "expected covar norm to be nil");
    f64_nil!(empty.stm.norm(), "expected STM norm to be nil");
    assert_eq!(empty.predicted, true, "expected predicted to be true");
}

#[test]
fn csv_serialize_empty_estimate() {
    use std::io;
    let empty = KfEstimate::zeros(State::zeros());
    let mut wtr = csv::Writer::from_writer(io::stdout());
    wtr.serialize(empty).expect("could not write to stdout");
}

#[test]
fn filter_errors() {
    let initial_estimate = KfEstimate::zeros(State::zeros());
    let process_noise = Matrix3::zeros();
    let measurement_noise = Matrix2::zeros();
    let real_obs = Vector2::zeros();
    let computed_obs = Vector2::zeros();
    let sensitivity = Matrix2x6::zeros();
    let stm = Matrix6::zeros();

    let mut ckf = KF::initialize(initial_estimate, process_noise, measurement_noise, None);
    match ckf.time_update(State::zeros()) {
        Ok(_) => panic!("expected the time update to fail"),
        Err(e) => {
            assert_eq!(e, FilterError::StateTransitionMatrixNotUpdated);
        }
    }
    match ckf.measurement_update(State::zeros(), real_obs, computed_obs) {
        Ok(_) => panic!("expected the measurement update to fail"),
        Err(e) => {
            assert_eq!(e, FilterError::StateTransitionMatrixNotUpdated);
        }
    }
    ckf.update_stm(stm);
    match ckf.measurement_update(State::zeros(), real_obs, computed_obs) {
        Ok(_) => panic!("expected the measurement update to fail"),
        Err(e) => {
            assert_eq!(e, FilterError::SensitivityNotUpdated);
        }
    }
    ckf.update_h_tilde(sensitivity);
    match ckf.measurement_update(State::zeros(), real_obs, computed_obs) {
        Ok(_) => panic!("expected the measurement update to fail"),
        Err(e) => {
            assert_eq!(e, FilterError::GainSingular);
        }
    }
}
