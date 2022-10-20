use ndarray::{Array2, ArrayViewMut2};

use super::RadialIntegral;
use crate::math::{HermitCubicSpline, SplineParameters};
use crate::Error;

/// `SplinedRadialIntegral` allows to evaluate another radial integral
/// implementation using [cubic Hermit spline][splines-wiki].
///
/// This can be much faster than using the actual radial integral
/// implementation.
///
/// [splines-wiki]: https://en.wikipedia.org/wiki/Cubic_Hermite_spline
pub struct SplinedRadialIntegral {
    spline: HermitCubicSpline<ndarray::Ix2>,
}

/// Parameters for computing the radial integral using Hermit cubic splines
#[derive(Debug, Clone, Copy)]
pub struct SplinedRIParameters {
    /// Number of radial components
    pub max_radial: usize,
    /// Number of angular components
    pub max_angular: usize,
    /// cutoff radius, this is also the maximal value that can be interpolated
    pub cutoff: f64,
}

impl SplinedRadialIntegral {
    /// Create a new `SplinedRadialIntegral` taking values from the given
    /// `radial_integral`. Points are added to the spline until the requested
    /// accuracy is reached. We consider that the accuracy is reached when
    /// either the mean absolute error or the mean relative error gets below the
    /// `accuracy` threshold.
    #[time_graph::instrument(name = "SplinedRadialIntegral::with_accuracy")]
    pub fn with_accuracy(
        parameters: SplinedRIParameters,
        accuracy: f64,
        radial_integral: impl RadialIntegral
    ) -> Result<SplinedRadialIntegral, Error> {
        let shape_tuple = (parameters.max_angular + 1, parameters.max_radial);

        let parameters = SplineParameters {
            start: 0.0,
            stop: parameters.cutoff,
            shape: vec![parameters.max_angular + 1, parameters.max_radial],
        };

        let spline = HermitCubicSpline::with_accuracy(
            accuracy,
            parameters,
            |x| {
                let mut values = Array2::from_elem(shape_tuple, 0.0);
                let mut gradients = Array2::from_elem(shape_tuple, 0.0);
                radial_integral.compute(x, values.view_mut(), Some(gradients.view_mut()));
                (values, gradients)
            },
        )?;

        return Ok(SplinedRadialIntegral { spline });
    }
}

impl RadialIntegral for SplinedRadialIntegral {
    #[time_graph::instrument(name = "SplinedRadialIntegral::compute")]
    fn compute(&self, x: f64, values: ArrayViewMut2<f64>, gradients: Option<ArrayViewMut2<f64>>) {
        self.spline.compute(x, values, gradients);
    }
}

#[cfg(test)]
mod tests {
    use approx::assert_relative_eq;

    use super::*;
    use super::super::super::soap::{SoapGtoRadialIntegral, GtoParameters};

    #[test]
    fn high_accuracy() {
        // Check that even with high accuracy and large domain MAX_SPLINE_SIZE
        // is enough
        let parameters = SplinedRIParameters {
            max_radial: 15,
            max_angular: 10,
            cutoff: 12.0,
        };

        let gto = SoapGtoRadialIntegral::new(GtoParameters {
            max_radial: parameters.max_radial,
            max_angular: parameters.max_angular,
            cutoff: parameters.cutoff,
            atomic_gaussian_width: 0.5,
        }).unwrap();

        // this test only check that this code runs without crashing
        SplinedRadialIntegral::with_accuracy(parameters, 1e-10, gto).unwrap();
    }

    #[test]
    fn finite_difference() {
        let max_radial = 8;
        let max_angular = 8;
        let parameters = SplinedRIParameters {
            max_radial: max_radial,
            max_angular: max_angular,
            cutoff: 5.0,
        };

        let gto = SoapGtoRadialIntegral::new(GtoParameters {
            max_radial: parameters.max_radial,
            max_angular: parameters.max_angular,
            cutoff: parameters.cutoff,
            atomic_gaussian_width: 0.5,
        }).unwrap();

        // even with very bad accuracy, we want the gradients of the spline to
        // match the values produces by the spline, and not necessarily the
        // actual GTO gradients.
        let spline = SplinedRadialIntegral::with_accuracy(parameters, 1e-2, gto).unwrap();

        let rij = 3.4;
        let delta = 1e-9;

        let shape = (max_angular + 1, max_radial);
        let mut values = Array2::from_elem(shape, 0.0);
        let mut values_delta = Array2::from_elem(shape, 0.0);
        let mut gradients = Array2::from_elem(shape, 0.0);
        spline.compute(rij, values.view_mut(), Some(gradients.view_mut()));
        spline.compute(rij + delta, values_delta.view_mut(), None);

        let finite_differences = (&values_delta - &values) / delta;
        assert_relative_eq!(
            finite_differences, gradients,
            epsilon=delta, max_relative=1e-6
        );
    }
}