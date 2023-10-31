use std::collections::BTreeSet;

use metatensor::{Labels, LabelsBuilder};

use crate::{System, Error};
use crate::systems::BATripletNeighborList;

/// Common interface to create a set of metatensor's `TensorMap` keys from systems
pub trait KeysBuilder {
    /// Compute the keys corresponding to these systems
    fn keys(&self, systems: &mut [System]) -> Result<Labels, Error>;
}

/// Compute a set of keys with a single variable, the central atom species.
pub struct CenterSpeciesKeys;

impl KeysBuilder for CenterSpeciesKeys {
    fn keys(&self, systems: &mut [System]) -> Result<Labels, Error> {
        let mut all_species = BTreeSet::new();
        for system in systems {
            for &species in system.species()? {
                all_species.insert(species);
            }
        }

        let mut keys = LabelsBuilder::new(vec!["species_center"]);
        for species in all_species {
            keys.add(&[species]);
        }
        return Ok(keys.finish());
    }
}

/// Compute a set of keys with two variables: the central atom species and a
/// all neighbor atom species within the whole system.
pub struct AllSpeciesPairsKeys {}

impl KeysBuilder for AllSpeciesPairsKeys {
    fn keys(&self, systems: &mut [System]) -> Result<Labels, Error> {

        let mut all_species_pairs = BTreeSet::new();
        for system in systems {
            for &species_first in system.species()? {
                for &species_second in system.species()? {
                    all_species_pairs.insert((species_first, species_second));
                }
            }
        }

        let mut keys = LabelsBuilder::new(vec!["species_center", "species_neighbor"]);
        for (center, neighbor) in all_species_pairs {
            keys.add(&[center, neighbor]);
        }

        return Ok(keys.finish());
    }
}

/// Compute a set of keys with two variables: the central atom species and a
/// single neighbor atom species within a cutoff around the central atom.
pub struct CenterSingleNeighborsSpeciesKeys {
    /// Spherical cutoff to use when searching for neighbors around an atom
    pub cutoff: f64,
    /// Should we consider an atom to be it's own neighbor or not?
    pub self_pairs: bool,
}

impl KeysBuilder for CenterSingleNeighborsSpeciesKeys {
    fn keys(&self, systems: &mut [System]) -> Result<Labels, Error> {
        assert!(self.cutoff > 0.0 && self.cutoff.is_finite());

        let mut all_species_pairs = BTreeSet::new();
        for system in systems {
            system.compute_neighbors(self.cutoff)?;

            let species = system.species()?;
            for pair in system.pairs()? {
                all_species_pairs.insert((species[pair.first], species[pair.second]));
                all_species_pairs.insert((species[pair.second], species[pair.first]));
            }

            if self.self_pairs {
                for &species in species {
                    all_species_pairs.insert((species, species));
                }
            }
        }

        let mut keys = LabelsBuilder::new(vec!["species_center", "species_neighbor"]);
        for (center, neighbor) in all_species_pairs {
            keys.add(&[center, neighbor]);
        }

        return Ok(keys.finish());
    }
}

/// Compute a set of keys with three variables: the species of two central atoms within a given cutoff to each other,
/// and the species of a third, neighbor atom, within a cutoff of the first two.
pub struct TwoCentersSingleNeighborsSpeciesKeys<'a> {
    /// Spherical cutoff to use when searching for neighbors around an atom
    pub(crate) cutoffs: [f64;2],
    /// Should we consider an atom to be it's own neighbor or not?
    pub self_contributions: bool,
    pub raw_triplets: &'a BATripletNeighborList,
}

impl<'a> TwoCentersSingleNeighborsSpeciesKeys<'a>{
    pub fn bond_cutoff(&self) -> f64 {
        self.cutoffs[0]
    }
    pub fn third_cutoff(&self) -> f64 {
        self.cutoffs[1]
    }
}


impl<'a> KeysBuilder for TwoCentersSingleNeighborsSpeciesKeys<'a> {
    fn keys(&self, systems: &mut [System]) -> Result<Labels, Error> {
        assert!(self.bond_cutoff() > 0.0 && self.bond_cutoff().is_finite() && self.third_cutoff() > 0.0 && self.third_cutoff().is_finite());

        let mut all_species_triplets = BTreeSet::new();
        for system in systems {
            self.raw_triplets.ensure_computed_for_system(system)?;

            let species = system.species()?;
            for triplet in self.raw_triplets.get_for_system(system, false)? {
                if (!self.self_contributions) && triplet.is_self_contrib {
                    continue;
                }
                all_species_triplets.insert((species[triplet.atom_i], species[triplet.atom_j], species[triplet.atom_k]));
                all_species_triplets.insert((species[triplet.atom_j], species[triplet.atom_i], species[triplet.atom_k]));
            }
        }

        let mut keys = LabelsBuilder::new(vec!["species_center_1", "species_center_2", "species_neighbor"]);
        for (center1, center2, neighbor) in all_species_triplets {
            keys.add(&[center1,center2, neighbor]);
        }

        return Ok(keys.finish());
    }
}

/// Compute a set of keys with three variables: the central atom species and two
/// neighbor atom species.
pub struct CenterTwoNeighborsSpeciesKeys {
    /// Spherical cutoff to use when searching for neighbors around an atom
    pub cutoff: f64,
    /// Should we consider an atom to be it's own neighbor or not?
    pub self_pairs: bool,
    /// Are neighbor atoms keys symmetric with respect to exchange or not?
    pub symmetric: bool,
}

impl KeysBuilder for CenterTwoNeighborsSpeciesKeys {
    fn keys(&self, systems: &mut [System]) -> Result<Labels, Error> {
        assert!(self.cutoff > 0.0 && self.cutoff.is_finite());

        let mut keys = BTreeSet::new();
        for system in systems {
            system.compute_neighbors(self.cutoff)?;
            let species = system.species()?;

            for center in 0..system.size()? {
                let species_center = species[center];

                // all neighbor species around the current center
                let mut neighbor_species = BTreeSet::new();
                for pair in system.pairs_containing(center)? {
                    let neighbor = if pair.first == center {
                        pair.second
                    } else {
                        debug_assert_eq!(pair.second, center);
                        pair.first
                    };

                    neighbor_species.insert(species[neighbor]);
                }

                if self.self_pairs {
                    neighbor_species.insert(species_center);
                }

                // create keys
                for &species_neighbor_1 in &neighbor_species {
                    for &species_neighbor_2 in &neighbor_species {
                        if self.symmetric && species_neighbor_2 < species_neighbor_1 {
                            continue;
                        }

                        keys.insert((species_center, species_neighbor_1, species_neighbor_2));
                    }
                }
            }
        }

        let mut keys_builder = LabelsBuilder::new(vec!["species_center", "species_neighbor_1", "species_neighbor_2"]);
        for (species_center, species_neighbor_1, species_neighbor_2) in keys {
            keys_builder.add(&[species_center, species_neighbor_1, species_neighbor_2]);
        }

        return Ok(keys_builder.finish());
    }
}
