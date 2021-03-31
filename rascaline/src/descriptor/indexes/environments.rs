use indexmap::IndexSet;

use crate::system::System;
use super::{Indexes, IndexesBuilder, EnvironmentIndexes, IndexValue};

/// `StructureEnvironment` is used to represents environments corresponding to
/// full structures, each structure being described by a single features vector.
///
/// It does not contain any chemical species information, for this you should
/// use `StructureSpeciesEnvironment`.
///
/// The base set of indexes contains only the `structure` index; the  gradient
/// indexes also contains the `atom` inside the structure with respect to which
/// the gradient is taken and the `spatial` (i.e. x/y/z) index.
pub struct StructureEnvironment;

impl EnvironmentIndexes for StructureEnvironment {
    fn names(&self) -> Vec<&str> {
        vec!["structure"]
    }

    #[time_graph::instrument(name = "StructureEnvironment::indexes")]
    fn indexes(&self, systems: &mut [&mut dyn System]) -> Indexes {
        let mut indexes = IndexesBuilder::new(self.names());
        for system in 0..systems.len() {
            indexes.add(&[IndexValue::from(system)]);
        }
        return indexes.finish();
    }

    #[time_graph::instrument(name = "StructureEnvironment::gradients_for")]
    fn gradients_for(&self, systems: &mut [&mut dyn System], samples: &Indexes) -> Option<Indexes> {
        assert_eq!(samples.names(), self.names());

        let mut gradients = IndexesBuilder::new(vec!["structure", "atom", "spatial"]);
        for value in samples.iter() {
            let system = value[0];
            for atom in 0..systems[system.usize()].size() {
                gradients.add(&[system, IndexValue::from(atom), IndexValue::from(0_usize)]);
                gradients.add(&[system, IndexValue::from(atom), IndexValue::from(1_usize)]);
                gradients.add(&[system, IndexValue::from(atom), IndexValue::from(2_usize)]);
            }
        }

        Some(gradients.finish())
    }
}

/// `AtomEnvironment` is used to represents atom-centered environments, where
/// each atom in a structure is described with a feature vector based on other
/// atoms inside a sphere centered on the central atom.
///
/// This type of indexes does not contain any chemical species information, for
/// this you should use `AtomSpeciesEnvironment`.
///
/// The base set of indexes contains `structure` and `center` (i.e. central atom
/// index inside the structure); the gradient indexes also contains the
/// `neighbor` inside the spherical cutoff with respect to which the gradient is
/// taken and the `spatial` (i.e x/y/z) index.
pub struct AtomEnvironment {
    /// spherical cutoff radius used to construct the atom-centered environments
    cutoff: f64,
}

impl AtomEnvironment {
    /// Create a new `AtomEnvironment` with the given cutoff.
    pub fn new(cutoff: f64) -> AtomEnvironment {
        assert!(cutoff > 0.0 && cutoff.is_finite(), "cutoff must be positive for AtomEnvironment");
        AtomEnvironment { cutoff }
    }
}

impl EnvironmentIndexes for AtomEnvironment {
    fn names(&self) -> Vec<&str> {
        vec!["structure", "center"]
    }

    #[time_graph::instrument(name = "AtomEnvironment::indexes")]
    fn indexes(&self, systems: &mut [&mut dyn System]) -> Indexes {
        let mut indexes = IndexesBuilder::new(self.names());
        for (i_system, system) in systems.iter().enumerate() {
            for center in 0..system.size() {
                indexes.add(&[IndexValue::from(i_system), IndexValue::from(center)]);
            }
        }
        return indexes.finish();
    }

    #[time_graph::instrument(name = "AtomEnvironment::gradients_for")]
    fn gradients_for(&self, systems: &mut [&mut dyn System], samples: &Indexes) -> Option<Indexes> {
        assert_eq!(samples.names(), self.names());

        // We need IndexSet to yield the indexes in the right order, i.e. the
        // order corresponding to whatever was passed in sample
        let mut indexes = IndexSet::new();
        for requested in samples {
            let i_system = requested[0];
            let center = requested[1].usize();
            let system = &mut *systems[i_system.usize()];
            system.compute_neighbors(self.cutoff);

            for pair in system.pairs_containing(center) {
                if pair.first == center {
                    indexes.insert((i_system, pair.first, pair.second));
                } else if pair.second == center {
                    indexes.insert((i_system, pair.second, pair.first));
                }
            }
        }

        let mut gradients = IndexesBuilder::new(vec!["structure", "center", "neighbor", "spatial"]);
        for (structure, atom, neighbor) in indexes {
            let atom = IndexValue::from(atom);
            let neighbor = IndexValue::from(neighbor);
            gradients.add(&[structure, atom, neighbor, IndexValue::from(0_usize)]);
            gradients.add(&[structure, atom, neighbor, IndexValue::from(1_usize)]);
            gradients.add(&[structure, atom, neighbor, IndexValue::from(2_usize)]);
        }

        return Some(gradients.finish());
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::system::test_systems;

    /// Convenience macro to create IndexValue
    macro_rules! v {
        ($value: expr) => {
            crate::descriptor::indexes::IndexValue::from($value as f64)
        };
    }

    #[test]
    fn structure() {
        let mut systems = test_systems(&["methane", "methane", "water"]);
        let indexes = StructureEnvironment.indexes(&mut systems.get());
        assert_eq!(indexes.count(), 3);
        assert_eq!(indexes.names(), &["structure"]);
        assert_eq!(indexes.iter().collect::<Vec<_>>(), vec![&[v!(0)], &[v!(1)], &[v!(2)]]);
    }

    #[test]
    fn structure_gradient() {
        let mut systems = test_systems(&["methane", "water"]);

        let (_, gradients) = StructureEnvironment.with_gradients(&mut systems.get());
        let gradients = gradients.unwrap();
        assert_eq!(gradients.count(), 24);
        assert_eq!(gradients.names(), &["structure", "atom", "spatial"]);
        assert_eq!(gradients.iter().collect::<Vec<_>>(), vec![
            // methane
            &[v!(0), v!(0), v!(0)], &[v!(0), v!(0), v!(1)], &[v!(0), v!(0), v!(2)],
            &[v!(0), v!(1), v!(0)], &[v!(0), v!(1), v!(1)], &[v!(0), v!(1), v!(2)],
            &[v!(0), v!(2), v!(0)], &[v!(0), v!(2), v!(1)], &[v!(0), v!(2), v!(2)],
            &[v!(0), v!(3), v!(0)], &[v!(0), v!(3), v!(1)], &[v!(0), v!(3), v!(2)],
            &[v!(0), v!(4), v!(0)], &[v!(0), v!(4), v!(1)], &[v!(0), v!(4), v!(2)],
            // water
            &[v!(1), v!(0), v!(0)], &[v!(1), v!(0), v!(1)], &[v!(1), v!(0), v!(2)],
            &[v!(1), v!(1), v!(0)], &[v!(1), v!(1), v!(1)], &[v!(1), v!(1), v!(2)],
            &[v!(1), v!(2), v!(0)], &[v!(1), v!(2), v!(1)], &[v!(1), v!(2), v!(2)],
        ]);
    }

    #[test]
    fn partial_structure_gradient() {
        let mut indexes = IndexesBuilder::new(vec!["structure"]);
        indexes.add(&[v!(2)]);
        indexes.add(&[v!(0)]);

        let mut systems = test_systems(&["water", "methane", "water", "methane"]);
        let gradients = StructureEnvironment.gradients_for(&mut systems.get(), &indexes.finish());
        let gradients = gradients.unwrap();

        assert_eq!(gradients.names(), &["structure", "atom", "spatial"]);
        assert_eq!(gradients.iter().collect::<Vec<_>>(), vec![
            // water #2
            &[v!(2), v!(0), v!(0)], &[v!(2), v!(0), v!(1)], &[v!(2), v!(0), v!(2)],
            &[v!(2), v!(1), v!(0)], &[v!(2), v!(1), v!(1)], &[v!(2), v!(1), v!(2)],
            &[v!(2), v!(2), v!(0)], &[v!(2), v!(2), v!(1)], &[v!(2), v!(2), v!(2)],
            // water #1
            &[v!(0), v!(0), v!(0)], &[v!(0), v!(0), v!(1)], &[v!(0), v!(0), v!(2)],
            &[v!(0), v!(1), v!(0)], &[v!(0), v!(1), v!(1)], &[v!(0), v!(1), v!(2)],
            &[v!(0), v!(2), v!(0)], &[v!(0), v!(2), v!(1)], &[v!(0), v!(2), v!(2)],
        ]);
    }

    #[test]
    fn atoms() {
        let mut systems = test_systems(&["methane", "water"]);
        let strategy = AtomEnvironment { cutoff: 2.0 };
        let indexes = strategy.indexes(&mut systems.get());
        assert_eq!(indexes.count(), 8);
        assert_eq!(indexes.names(), &["structure", "center"]);
        assert_eq!(indexes.iter().collect::<Vec<_>>(), vec![
            &[v!(0), v!(0)], &[v!(0), v!(1)], &[v!(0), v!(2)], &[v!(0), v!(3)], &[v!(0), v!(4)],
            &[v!(1), v!(0)], &[v!(1), v!(1)], &[v!(1), v!(2)],
        ]);
    }

    #[test]
    fn atom_gradients() {
        let mut systems = test_systems(&["methane"]);
        let strategy = AtomEnvironment { cutoff: 1.5 };
        let (_, gradients) = strategy.with_gradients(&mut systems.get());
        let gradients = gradients.unwrap();

        assert_eq!(gradients.count(), 24);
        assert_eq!(gradients.names(), &["structure", "center", "neighbor", "spatial"]);
        assert_eq!(gradients.iter().collect::<Vec<_>>(), vec![
            // Only C-H neighbors are within 1.3 A
            // C center
            &[v!(0), v!(0), v!(1), v!(0)],
            &[v!(0), v!(0), v!(1), v!(1)],
            &[v!(0), v!(0), v!(1), v!(2)],

            &[v!(0), v!(0), v!(2), v!(0)],
            &[v!(0), v!(0), v!(2), v!(1)],
            &[v!(0), v!(0), v!(2), v!(2)],

            &[v!(0), v!(0), v!(3), v!(0)],
            &[v!(0), v!(0), v!(3), v!(1)],
            &[v!(0), v!(0), v!(3), v!(2)],

            &[v!(0), v!(0), v!(4), v!(0)],
            &[v!(0), v!(0), v!(4), v!(1)],
            &[v!(0), v!(0), v!(4), v!(2)],
            // H centers
            &[v!(0), v!(1), v!(0), v!(0)],
            &[v!(0), v!(1), v!(0), v!(1)],
            &[v!(0), v!(1), v!(0), v!(2)],

            &[v!(0), v!(2), v!(0), v!(0)],
            &[v!(0), v!(2), v!(0), v!(1)],
            &[v!(0), v!(2), v!(0), v!(2)],

            &[v!(0), v!(3), v!(0), v!(0)],
            &[v!(0), v!(3), v!(0), v!(1)],
            &[v!(0), v!(3), v!(0), v!(2)],

            &[v!(0), v!(4), v!(0), v!(0)],
            &[v!(0), v!(4), v!(0), v!(1)],
            &[v!(0), v!(4), v!(0), v!(2)],
        ]);
    }

    #[test]
    fn partial_atom_gradient() {
        let mut indexes = IndexesBuilder::new(vec!["structure", "center"]);
        // out of order values to ensure the gradients are also out of order
        indexes.add(&[v!(0), v!(2)]);
        indexes.add(&[v!(0), v!(0)]);

        let mut systems = test_systems(&["methane"]);
        let strategy = AtomEnvironment { cutoff: 1.5 };
        let gradients = strategy.gradients_for(&mut systems.get(), &indexes.finish());
        let gradients = gradients.unwrap();

        assert_eq!(gradients.names(), &["structure", "center", "neighbor", "spatial"]);
        assert_eq!(gradients.iter().collect::<Vec<_>>(), vec![
            // H centers
            &[v!(0), v!(2), v!(0), v!(0)],
            &[v!(0), v!(2), v!(0), v!(1)],
            &[v!(0), v!(2), v!(0), v!(2)],
            // C center
            &[v!(0), v!(0), v!(1), v!(0)],
            &[v!(0), v!(0), v!(1), v!(1)],
            &[v!(0), v!(0), v!(1), v!(2)],

            &[v!(0), v!(0), v!(2), v!(0)],
            &[v!(0), v!(0), v!(2), v!(1)],
            &[v!(0), v!(0), v!(2), v!(2)],

            &[v!(0), v!(0), v!(3), v!(0)],
            &[v!(0), v!(0), v!(3), v!(1)],
            &[v!(0), v!(0), v!(3), v!(2)],

            &[v!(0), v!(0), v!(4), v!(0)],
            &[v!(0), v!(0), v!(4), v!(1)],
            &[v!(0), v!(0), v!(4), v!(2)],
        ]);
    }
}
