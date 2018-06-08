extern crate nalgebra as na;
use self::na::{Matrix6, MatrixMN, U3, U36, U42, U6, Vector6, VectorN};
use super::Dynamics;
use celestia::{CelestialBody, CoordinateFrame, State};
use od::Linearization;
use std::f64;
use std::sync::mpsc::Sender;

/// `TwoBody` exposes the equations of motion for a simple two body propagation.
#[derive(Clone)]
pub struct TwoBody<'a> {
    pub mu: f64,
    pub tx_chan: Option<&'a Sender<(f64, Vector6<f64>)>>,
    time: f64,
    pos_vel: Vector6<f64>,
}

impl<'a> TwoBody<'a> {
    /// Initialize TwoBody dynamics given a provided gravitional parameter (as `mu`)
    pub fn from_state_vec_with_gm(state: Vector6<f64>, mu: f64) -> TwoBody<'a> {
        TwoBody {
            mu: mu,
            tx_chan: None,
            time: 0.0,
            pos_vel: state,
        }
    }

    /// Initialize TwoBody dynamics around a provided `CelestialBody` from the provided state vector (cf. nyx::celestia).
    pub fn from_state_vec<B: CelestialBody>(state: Vector6<f64>) -> TwoBody<'a> {
        TwoBody {
            mu: B::gm(),
            tx_chan: None,
            time: 0.0,
            pos_vel: state,
        }
    }
}

impl<'a> Dynamics for TwoBody<'a> {
    type StateSize = U6;
    fn time(&self) -> f64 {
        self.time
    }

    fn state(&self) -> VectorN<f64, Self::StateSize> {
        self.pos_vel
    }

    fn set_state(&mut self, new_t: f64, new_state: &VectorN<f64, Self::StateSize>) {
        self.time = new_t;
        self.pos_vel = *new_state;
        match self.tx_chan {
            Some(ref chan) => match chan.send((new_t, new_state.clone())) {
                Err(e) => warn!("could not publish to channel: {}", e),
                Ok(_) => {}
            },
            _ => {}
        }
    }

    fn eom(&self, _t: f64, state: &VectorN<f64, Self::StateSize>) -> VectorN<f64, Self::StateSize> {
        let radius = state.fixed_rows::<U3>(0).into_owned();
        let velocity = state.fixed_rows::<U3>(3).into_owned();
        let body_acceleration = (-self.mu / radius.norm().powi(3)) * radius;
        Vector6::from_iterator(velocity.iter().chain(body_acceleration.iter()).cloned())
    }
}

/// `TwoBody` exposes the equations of motion for a simple two body propagation. It inherently supports
/// the State Transition Matrix for orbital determination.
#[derive(Clone)]
pub struct TwoBodyWithStm<'a> {
    pub stm: Matrix6<f64>,
    pub two_body_dyn: TwoBody<'a>,
}

impl<'a> TwoBodyWithStm<'a> {
    /// Initialize TwoBody dynamics around a provided `CelestialBody` from the provided position and velocity state (cf. nyx::celestia).
    pub fn from_state<B: CelestialBody, F: CoordinateFrame>(state: State<F>) -> TwoBodyWithStm<'a> {
        TwoBodyWithStm {
            stm: Matrix6::identity(),
            two_body_dyn: TwoBody::from_state_vec::<B>(state.to_cartesian_vec()),
        }
    }
}

impl<'a> Dynamics for TwoBodyWithStm<'a> {
    type StateSize = U42;
    fn time(&self) -> f64 {
        self.two_body_dyn.time()
    }

    fn state(&self) -> VectorN<f64, Self::StateSize> {
        let mut stm_as_vec = VectorN::<f64, U36>::zeros();
        let mut stm_idx = 0;
        for i in 0..6 {
            for j in 0..6 {
                stm_as_vec[(stm_idx, 0)] = self.stm[(i, j)];
                stm_idx += 1;
            }
        }
        VectorN::<f64, Self::StateSize>::from_iterator(self.two_body_dyn.state().iter().chain(stm_as_vec.iter()).cloned())
    }

    fn set_state(&mut self, new_t: f64, new_state: &VectorN<f64, Self::StateSize>) {
        self.two_body_dyn
            .set_state(new_t, &new_state.fixed_rows::<U6>(0).into_owned());
        let mut stm_k_to_0 = Matrix6::zeros();
        let mut stm_idx = 6;
        for i in 0..6 {
            for j in 0..6 {
                stm_k_to_0[(i, j)] = new_state[(stm_idx, 0)];
                stm_idx += 1;
            }
        }
        let mut stm_prev = self.stm.clone();
        if !stm_prev.try_inverse_mut() {
            panic!("STM not invertible");
        }
        self.stm = stm_k_to_0 * stm_prev;
    }

    fn eom(&self, t: f64, state: &VectorN<f64, Self::StateSize>) -> VectorN<f64, Self::StateSize> {
        let pos_vel = state.fixed_rows::<U6>(0).into_owned();
        let two_body_dt = self.two_body_dyn.eom(t, &pos_vel);
        let stm_dt = self.stm * self.gradient(t, &pos_vel);
        // Rebuild the STM as a vector.
        let mut stm_as_vec = VectorN::<f64, U36>::zeros();
        let mut stm_idx = 0;
        for i in 0..6 {
            for j in 0..6 {
                stm_as_vec[(stm_idx, 0)] = stm_dt[(i, j)];
                stm_idx += 1;
            }
        }
        VectorN::<f64, Self::StateSize>::from_iterator(two_body_dt.iter().chain(stm_as_vec.iter()).cloned())
    }
}

impl<'a> Linearization for TwoBodyWithStm<'a> {
    type StateSize = U6;

    fn gradient(&self, _t: f64, state: &VectorN<f64, Self::StateSize>) -> MatrixMN<f64, Self::StateSize, Self::StateSize> {
        let mut grad = MatrixMN::<f64, U6, U6>::zeros();
        // Top right is Identity 3x3
        grad[(0, 3)] = 1.0;
        grad[(1, 4)] = 1.0;
        grad[(2, 5)] = 1.0;
        // Bottom left is where the magic happens.
        let x = state[(0, 0)];
        let y = state[(1, 0)];
        let z = state[(2, 0)];
        let x2 = x.powi(2);
        let y2 = y.powi(2);
        let z2 = z.powi(2);
        let r232 = (x2 + y2 + z2).powf(3.0 / 2.0);
        let r252 = (x2 + y2 + z2).powf(5.0 / 2.0);

        // Add the body perturbations
        let dax_dx = 3.0 * self.two_body_dyn.mu * x2 / r252 - self.two_body_dyn.mu / r232;
        let dax_dy = 3.0 * self.two_body_dyn.mu * x * y / r252;
        let dax_dz = 3.0 * self.two_body_dyn.mu * x * z / r252;
        let day_dx = 3.0 * self.two_body_dyn.mu * x * y / r252;
        let day_dy = 3.0 * self.two_body_dyn.mu * y2 / r252 - self.two_body_dyn.mu / r232;
        let day_dz = 3.0 * self.two_body_dyn.mu * y * z / r252;
        let daz_dx = 3.0 * self.two_body_dyn.mu * x * z / r252;
        let daz_dy = 3.0 * self.two_body_dyn.mu * y * z / r252;
        let daz_dz = 3.0 * self.two_body_dyn.mu * z2 / r252 - self.two_body_dyn.mu / r232;

        // Let the gradient
        grad[(3, 0)] = dax_dx;
        grad[(4, 0)] = day_dx;
        grad[(5, 0)] = daz_dx;
        grad[(3, 1)] = dax_dy;
        grad[(4, 1)] = day_dy;
        grad[(5, 1)] = daz_dy;
        grad[(3, 2)] = dax_dz;
        grad[(4, 2)] = day_dz;
        grad[(5, 2)] = daz_dz;

        grad
    }
}
