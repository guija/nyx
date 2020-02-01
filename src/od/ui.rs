extern crate nalgebra as na;

use self::na::allocator::Allocator;
use self::na::DefaultAllocator;

pub use super::estimate::*;
pub use super::kalman::*;
pub use super::ranging::*;
pub use super::residual::*;
pub use super::*;

use crate::propagators::error_ctrl::ErrorCtrl;
use crate::propagators::Propagator;

pub struct ODProcess<
    'a,
    D: Estimable<N::MeasurementInput, LinStateSize = M::StateSize>,
    E: ErrorCtrl,
    M: Measurement,
    N: MeasurementDevice<M>,
    T: EkfTrigger,
> where
    DefaultAllocator: Allocator<f64, D::StateSize>
        + Allocator<f64, M::MeasurementSize>
        + Allocator<f64, M::MeasurementSize, M::StateSize>
        + Allocator<f64, D::LinStateSize>
        + Allocator<f64, M::MeasurementSize, M::MeasurementSize>
        + Allocator<f64, M::MeasurementSize, D::LinStateSize>
        + Allocator<f64, D::LinStateSize, M::MeasurementSize>
        + Allocator<f64, D::LinStateSize, D::LinStateSize>,
{
    /// Propagator used for the estimation
    pub prop: &'a mut Propagator<'a, D, E>,
    /// Kalman filter itself
    pub kf: &'a mut KF<D::LinStateSize, M::MeasurementSize>,
    /// List of measurement devices used
    pub devices: &'a [N],
    /// Whether or not these devices can make simultaneous measurements of the spacecraft
    pub simultaneous_msr: bool,
    /// Vector of estimates available after a pass
    pub estimates: Vec<Estimate<D::LinStateSize>>,
    /// Vector of residuals available after a pass
    pub residuals: Vec<Residual<M::MeasurementSize>>,
    pub ekf_trigger: T,
}

impl<
        'a,
        D: Estimable<N::MeasurementInput, LinStateSize = M::StateSize>,
        E: ErrorCtrl,
        M: Measurement,
        N: MeasurementDevice<M>,
        T: EkfTrigger,
    > ODProcess<'a, D, E, M, N, T>
where
    DefaultAllocator: Allocator<f64, D::StateSize>
        + Allocator<f64, M::MeasurementSize>
        + Allocator<f64, M::MeasurementSize, M::StateSize>
        + Allocator<f64, D::LinStateSize>
        + Allocator<f64, M::MeasurementSize, M::MeasurementSize>
        + Allocator<f64, M::MeasurementSize, D::LinStateSize>
        + Allocator<f64, D::LinStateSize, M::MeasurementSize>
        + Allocator<f64, D::LinStateSize, D::LinStateSize>,
{
    pub fn ekf(
        prop: &'a mut Propagator<'a, D, E>,
        kf: &'a mut KF<D::LinStateSize, M::MeasurementSize>,
        devices: &'a [N],
        simultaneous_msr: bool,
        num_expected_msr: usize,
        trigger: T,
    ) -> Self {
        Self {
            prop,
            kf,
            devices: &devices,
            simultaneous_msr,
            estimates: Vec::with_capacity(num_expected_msr),
            residuals: Vec::with_capacity(num_expected_msr),
            ekf_trigger: trigger,
        }
    }

    pub fn default_ekf(
        prop: &'a mut Propagator<'a, D, E>,
        kf: &'a mut KF<D::LinStateSize, M::MeasurementSize>,
        devices: &'a [N],
        trigger: T,
    ) -> Self {
        Self {
            prop,
            kf,
            devices: &devices,
            simultaneous_msr: false,
            estimates: Vec::with_capacity(10_000),
            residuals: Vec::with_capacity(10_000),
            ekf_trigger: trigger,
        }
    }

    /// Allows to smooth the provided estimates. Returns an array of smoothed estimates.
    ///
    /// Estimates must be ordered in chronological order. This function will smooth the
    /// estimates from the last in the list to the first one.
    pub fn smooth(&mut self) -> Option<FilterError> {
        debug!("Smoothing {} estimates", self.estimates.len());
        let mut smoothed = Vec::with_capacity(self.estimates.len());

        for estimate in self.estimates.iter().rev() {
            let mut sm_est = estimate.clone();
            // TODO: Ensure that SNC was _not_ enabled
            let mut stm_inv = estimate.stm.clone();
            if !stm_inv.try_inverse_mut() {
                return Some(FilterError::StateTransitionMatrixSingular);
            }
            sm_est.covar = &stm_inv * &estimate.covar * &stm_inv.transpose();
            sm_est.state = &stm_inv * &estimate.state;
            smoothed.push(sm_est);
        }

        // And reverse to maintain order
        smoothed.reverse();
        // And store
        self.estimates = smoothed;

        None
    }

    /// Allows processing all measurements without covariance mapping. Only works for CKFs.
    pub fn process_measurements(&mut self, measurements: &[(Epoch, M)]) -> Option<FilterError> {
        info!("Processing {} measurements", measurements.len());

        let mut prev_dt = self.kf.prev_estimate.dt;

        for (next_epoch, real_meas) in measurements.iter() {
            // Propagate the dynamics to the measurement, and then start the filter.
            let delta_time = *next_epoch - prev_dt;
            prev_dt = *next_epoch; // Update the epoch for the next computation
            self.prop.until_time_elapsed(delta_time);
            // Update the STM of the KF
            self.kf.update_stm(self.prop.dynamics.stm());
            let (dt, meas_input) = self
                .prop
                .dynamics
                .to_measurement(&self.prop.dynamics.state());
            // Get the computed observations
            for device in self.devices.iter() {
                let computed_meas: M = device.measure(&meas_input);
                if computed_meas.visible() {
                    self.kf.update_h_tilde(computed_meas.sensitivity());
                    match self.kf.measurement_update(
                        dt,
                        real_meas.observation(),
                        computed_meas.observation(),
                    ) {
                        Ok((est, res)) => {
                            // Switch to EKF if necessary, and update the dynamics and such
                            if !self.kf.ekf && self.ekf_trigger.enable_ekf(&est) {
                                self.kf.ekf = true;
                                info!("EKF now enabled");
                            }
                            if self.kf.ekf {
                                let est_state = est.state.clone();
                                self.prop.dynamics.set_estimated_state(
                                    self.prop.dynamics.estimated_state() + est_state,
                                );
                            }
                            self.estimates.push(est);
                            self.residuals.push(res);
                        }
                        Err(e) => return Some(e),
                    }
                    if !self.simultaneous_msr {
                        break;
                    }
                }
            }
        }

        None
    }
}

impl<
        'a,
        D: Estimable<N::MeasurementInput, LinStateSize = M::StateSize>,
        E: ErrorCtrl,
        M: Measurement,
        N: MeasurementDevice<M>,
    > ODProcess<'a, D, E, M, N, CkfTrigger>
where
    DefaultAllocator: Allocator<f64, D::StateSize>
        + Allocator<f64, M::MeasurementSize>
        + Allocator<f64, M::MeasurementSize, M::StateSize>
        + Allocator<f64, D::LinStateSize>
        + Allocator<f64, M::MeasurementSize, M::MeasurementSize>
        + Allocator<f64, M::MeasurementSize, D::LinStateSize>
        + Allocator<f64, D::LinStateSize, M::MeasurementSize>
        + Allocator<f64, D::LinStateSize, D::LinStateSize>,
{
    pub fn ckf(
        prop: &'a mut Propagator<'a, D, E>,
        kf: &'a mut KF<D::LinStateSize, M::MeasurementSize>,
        devices: &'a [N],
        simultaneous_msr: bool,
        num_expected_msr: usize,
    ) -> Self {
        Self {
            prop,
            kf,
            devices: &devices,
            simultaneous_msr,
            estimates: Vec::with_capacity(num_expected_msr),
            residuals: Vec::with_capacity(num_expected_msr),
            ekf_trigger: CkfTrigger {},
        }
    }

    pub fn default_ckf(
        prop: &'a mut Propagator<'a, D, E>,
        kf: &'a mut KF<D::LinStateSize, M::MeasurementSize>,
        devices: &'a [N],
    ) -> Self {
        Self {
            prop,
            kf,
            devices: &devices,
            simultaneous_msr: false,
            estimates: Vec::with_capacity(10_000),
            residuals: Vec::with_capacity(10_000),
            ekf_trigger: CkfTrigger {},
        }
    }
}
/// A trait detailing when to switch to from a CKF to an EKF
pub trait EkfTrigger {
    fn enable_ekf<S>(&mut self, est: &Estimate<S>) -> bool
    where
        S: DimName,
        DefaultAllocator: Allocator<f64, S> + Allocator<f64, S, S>;
}

/// CkfTrigger will never switch a KF to an EKF
pub struct CkfTrigger;

impl EkfTrigger for CkfTrigger {
    fn enable_ekf<S>(&mut self, _est: &Estimate<S>) -> bool
    where
        S: DimName,
        DefaultAllocator: Allocator<f64, S> + Allocator<f64, S, S>,
    {
        false
    }
}

/// An EkfTrigger on the number of measurements processed
pub struct NumMsrEkfTrigger {
    pub num_msrs: usize,
    cur_msrs: usize,
}

impl NumMsrEkfTrigger {
    pub fn init(num_msrs: usize) -> Self {
        Self {
            num_msrs,
            cur_msrs: 0,
        }
    }
}

impl EkfTrigger for NumMsrEkfTrigger {
    fn enable_ekf<S>(&mut self, _est: &Estimate<S>) -> bool
    where
        S: DimName,
        DefaultAllocator: Allocator<f64, S> + Allocator<f64, S, S>,
    {
        self.cur_msrs += 1;
        self.cur_msrs >= self.num_msrs
    }
}