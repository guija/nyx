use crate::cosmic::{Cosm, Frame, Orbit};
use crate::dynamics::AccelModel;
use crate::io::gravity::HarmonicsMem;
use crate::linalg::{DMatrix, Matrix3, Vector3, U7};
use hyperdual::linalg::norm;
use hyperdual::{hyperspace_from_vector, Float, OHyperdual};
use num::integer::Roots;
use std::cmp::{max, min};
use std::fmt;
use std::sync::Arc;

use super::DynamicsError;

#[derive(Clone)]
pub struct Harmonics2 {
    cosm: Arc<Cosm>,
    compute_frame: Frame,
    stor: HarmonicsMem,
    // a_nm: DMatrix<f64>,
    // b_nm: DMatrix<f64>,
    // c_nm: DMatrix<f64>,
    // vr01: DMatrix<f64>,
    // vr11: DMatrix<f64>,
    // a_nm_h: DMatrix<OHyperdual<f64, U7>>,
    // b_nm_h: DMatrix<OHyperdual<f64, U7>>,
    // c_nm_h: DMatrix<OHyperdual<f64, U7>>,
    // vr01_h: DMatrix<OHyperdual<f64, U7>>,
    // vr11_h: DMatrix<OHyperdual<f64, U7>>,
}

impl Harmonics2 {
    pub fn from_stor(compute_frame: Frame, stor: HarmonicsMem, cosm: Arc<Cosm>) -> Arc<Self> {
        // the pre-computed arrays hold coefficients from triangular arrays in a single
        // storing neither diagonal elements (n = m) nor the non-diagonal element n=1, m=0
        let degree = stor.max_degree_n() as i32;
        let size = max(0, degree * (degree + 1) / 2 - 1);
        // TODO GJA research how to use a vector
        let mut gnmOj = DMatrix::zeros(size as usize, 1);
        let mut hnmOj = DMatrix::zeros(size as usize, 1);
        let mut enm = DMatrix::zeros(size as usize, 1);
        let mut index = 0;
        let mut m = degree;

        // pre-compute the recursion coefficients corresponding to equations 19 and 22
        // from Holmes and Featherstone paper
        // for cache efficiency, elements are stored in the same order they will be used
        // later on, i.e. from rightmost column to leftmost column
        while m >= 0 {
            let j = if m == 0 { 2 } else { 1 };
            let mut n = max(2, m + 1);
            while n <= degree {
                let f = (n - m) * (n + m + 1);
                gnmOj[index] = 2 * (m + 1) / (j * f).sqrt();
                hnmOj[index] = ((n + m + 2) * (n - m - 1) / (j * f)).sqrt();
                enm[index] = (f / j).sqrt();
                index += 1;
                n += 1;
            }
            m -= 1;
        }

        let mut sectorial = DMatrix::zeros((degree + 1) as usize, 1);
        let scaling = 930.0 as f64;
        let two = 2.0 as f64;
        sectorial[0] = 1.0 * two.powf(-scaling);

        Arc::new(Self {
            cosm,
            compute_frame,
            stor,
            // TODO add computed coefficients
        })
    }
}

// Harmonics implements AccelModel
impl AccelModel for Harmonics2 {
    fn eom(&self, osc: &Orbit) -> Result<Vector3<f64>, DynamicsError> {
        // TODO
        todo!()
    }

    fn dual_eom(&self, osc: &Orbit) -> Result<(Vector3<f64>, Matrix3<f64>), DynamicsError> {
        // TODO
        todo!()
    }
}

impl fmt::Display for Harmonics2 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{} gravity field {}x{} (order x degree)",
            self.compute_frame,
            self.stor.max_order_m(),
            self.stor.max_degree_n(),
        )
    }
}
