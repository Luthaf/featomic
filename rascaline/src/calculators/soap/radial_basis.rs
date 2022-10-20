use crate::calculators::radial_integral::RadialIntegral;
use crate::calculators::radial_integral::{SplinedRadialIntegral, SplinedRIParameters};

use super::{SoapGtoRadialIntegral, GtoParameters};
use super::SphericalExpansionParameters;

use crate::Error;

#[derive(Debug, Clone, Copy)]
#[derive(serde::Deserialize, serde::Serialize, schemars::JsonSchema)]
/// Radial basis that can be used in the SOAP spherical expansion
pub enum SoapRadialBasis {
    /// Use a radial basis similar to Gaussian-Type Orbitals.
    ///
    /// The basis is defined as `R_n(r) ∝ r^n e^{- r^2 / (2 σ_n^2)}`, where `σ_n
    /// = cutoff * \sqrt{n} / n_max`.
    ///
    /// If `splined_radial_integral` is true, we compute the radial integral
    /// using splines. This is much faster than the base GTO implementation. In
    /// this case, the number of control points in the spline is automatically
    /// determined to ensure the average absolute error is close to the
    /// `spline_accuracy`
    Gto {
        #[serde(default = "serde_default_splined_radial_integral")]
        splined_radial_integral: bool,
        #[serde(default = "serde_default_spline_accuracy")]
        spline_accuracy: f64,
    },
}

fn serde_default_splined_radial_integral() -> bool { true }
fn serde_default_spline_accuracy() -> f64 { 1e-8 }

impl SoapRadialBasis {
    /// Use GTO as the radial basis, and do not spline the radial integral
    pub fn gto() -> SoapRadialBasis {
        return SoapRadialBasis::Gto {
            splined_radial_integral: false, spline_accuracy: 0.0
        };
    }

    /// Use GTO as the radial basis, and spline the radial integral
    pub fn splined_gto(accuracy: f64) -> SoapRadialBasis {
        return SoapRadialBasis::Gto{
            splined_radial_integral: true, spline_accuracy: accuracy
        };
    }

    /// Construct the right SOAP radial integral for the current radial basis &
    /// set of spherical expansion parameters.
    pub fn get_radial_integral(&self, parameters: &SphericalExpansionParameters) -> Result<Box<dyn RadialIntegral>, Error> {
        match self {
            SoapRadialBasis::Gto {splined_radial_integral, spline_accuracy} => {
                let parameters = GtoParameters {
                    max_radial: parameters.max_radial,
                    max_angular: parameters.max_angular,
                    atomic_gaussian_width: parameters.atomic_gaussian_width,
                    cutoff: parameters.cutoff,
                };
                let gto = SoapGtoRadialIntegral::new(parameters)?;

                if !splined_radial_integral {
                    return Ok(Box::new(gto));
                }

                let parameters = SplinedRIParameters {
                    max_radial: parameters.max_radial,
                    max_angular: parameters.max_angular,
                    cutoff: parameters.cutoff,
                };

                return Ok(Box::new(SplinedRadialIntegral::with_accuracy(
                    parameters, *spline_accuracy, gto
                )?));
            }
        };
    }
}