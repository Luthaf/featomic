/// Euler's constant
pub const EULER: f64 = 0.57721566490153286;

mod gamma;
pub use self::gamma::{gamma, ln_gamma, digamma};

mod hyp1f1;
pub use self::hyp1f1::hyp1f1;

mod hyp2f1;
pub use self::hyp2f1::hyp2f1;

mod double_regularized_1f1;
pub use self::double_regularized_1f1::DoubleRegularized1F1;

mod eigen;
pub use self::eigen::SymmetricEigen;
