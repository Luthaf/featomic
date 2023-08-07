import json
from math import sqrt
from typing import List, Optional, Union

import numpy as np
from equistore.core import Labels, TensorBlock, TensorMap

from ..calculators import CalculatorBase
from ..systems import IntoSystem


class PowerSpectrum:
    def __init__(
        self,
        calculator_1: CalculatorBase,
        calculator_2: Optional[CalculatorBase] = None,
    ):
        r"""General power spectrum of one or of two calculators.

        If ``calculator_2`` is provided, the invariants :math:`p_{nl}` are generated by
        taking quadratic combinations of ``calculator_1``'s spherical expansion
        :math:`\rho_{nlm}` and ``calculator_2``'s spherical expansion :math:`\nu_{nlm}`
        according to `Bartók et. al
        <https://journals.aps.org/prb/abstract/10.1103/PhysRevB.87.184115>`_.

        .. math::
            p_{nl} = \rho_{nlm}^\dagger \cdot \nu_{nlm}

        where we use the Einstein summation convention. If gradients are present the
        invariants of those are constructed as

        .. math::
            \nabla p_{nl} = \nabla \rho_{nlm}^\dagger \cdot \nu_{nlm} +
                            \rho_{nlm}^\dagger \cdot \nabla \nu_{nlm}

        .. note::
            Currently only supports gradients with respect to ``positions``.

        If ``calculator_2=None`` invariants are generated by combining the coefficients
        of the spherical expansion of ``calculator_1``. The spherical expansions given
        as input can only be :py:class:`rascaline.SphericalExpansion` or
        :py:class:`rascaline.LodeSphericalExpansion`.

        :param calculator_1: first calculator
        :param calculator_1: second calculator
        :raises ValueError: If other calculators than
            :py:class:`rascaline.SphericalExpansion` or
            :py:class:`rascaline.LodeSphericalExpansion` are used.
        :raises ValueError: If ``'max_angular'`` of both calculators is different

        Example
        -------
        As an example we calculate the power spectrum for a short range (sr) spherical
        expansion and a long-range (lr) LODE spherical expansion for a NaCl crystal.

        >>> import rascaline
        >>> import ase

        Construct the NaCl crystal

        >>> atoms = ase.Atoms(
        ...     symbols="NaCl",
        ...     positions=[[0, 0, 0], [0.5, 0.5, 0.5]],
        ...     pbc=True,
        ...     cell=[1, 1, 1],
        ... )

        Define the hyper parameters for the short-range spherical expansion

        >>> sr_hypers = {
        ...     "cutoff": 1.0,
        ...     "max_radial": 6,
        ...     "max_angular": 2,
        ...     "atomic_gaussian_width": 0.3,
        ...     "center_atom_weight": 1.0,
        ...     "radial_basis": {
        ...         "Gto": {},
        ...     },
        ...     "cutoff_function": {
        ...         "ShiftedCosine": {"width": 0.5},
        ...     },
        ... }

        Define the hyper parameters for the long-range LODE spherical expansion from the
        hyper parameters of the short-range spherical expansion

        >>> lr_hypers = sr_hypers.copy()
        >>> lr_hypers.pop("cutoff_function")
        {'ShiftedCosine': {'width': 0.5}}
        >>> lr_hypers["potential_exponent"] = 1

        Construct the calculators

        >>> sr_calculator = rascaline.SphericalExpansion(**sr_hypers)
        >>> lr_calculator = rascaline.LodeSphericalExpansion(**lr_hypers)

        Construct the power spectrum calculators and compute the spherical expansion

        >>> calculator = rascaline.utils.PowerSpectrum(sr_calculator, lr_calculator)
        >>> power_spectrum = calculator.compute(atoms)

        The resulting invariants are stored as :py:class:`equistore.TensorMap` as for any other calculator

        >>> power_spectrum.keys
        Labels(
            species_center
                  11
                  17
        )
        >>> power_spectrum[0]
        TensorBlock
            samples (1): ['structure', 'center']
            components (): []
            properties (432): ['l', 'species_neighbor_1', 'n1', 'species_neighbor_2', 'n2']
            gradients: None


        .. seealso::
            If you are interested in the SOAP power spectrum you can the use the
            faster :py:class:`rascaline.SoapPowerSpectrum`.
        """  # noqa E501
        self.calculator_1 = calculator_1
        self.calculator_2 = calculator_2

        supported_calculators = ["lode_spherical_expansion", "spherical_expansion"]

        if self.calculator_1.c_name not in supported_calculators:
            raise ValueError(
                f"Only {','.join(supported_calculators)} are supported for "
                "calculator_1!"
            )

        if self.calculator_2 is not None:
            if self.calculator_2.c_name not in supported_calculators:
                raise ValueError(
                    f"Only {','.join(supported_calculators)} are supported for "
                    "calculator_2!"
                )

            parameters_1 = json.loads(calculator_1.parameters)
            parameters_2 = json.loads(calculator_2.parameters)
            if parameters_1["max_angular"] != parameters_2["max_angular"]:
                raise ValueError("'max_angular' of both calculators must be the same!")

    @property
    def name(self):
        """Name of this calculator."""
        return "PowerSpectrum"

    def compute(
        self,
        systems: Union[IntoSystem, List[IntoSystem]],
        *,
        gradients: Optional[List[str]] = None,
        use_native_system: bool = True,
    ) -> TensorMap:
        """Runs a calculation with this calculator on the given ``systems``.

        See :py:func:`rascaline.calculators.CalculatorBase.compute()` for details on the
        parameters.

        :raises NotImplementedError: If a spherical expansions contains a gradient with
            respect to an unknwon parameter.
        """
        if gradients is not None:
            for parameter in gradients:
                if parameter != "positions":
                    raise NotImplementedError(
                        "PowerSpectrum currently only supports gradients "
                        "w.r.t. to positions"
                    )

        spherical_expansion_1 = self.calculator_1.compute(
            systems=systems,
            gradients=gradients,
            use_native_system=use_native_system,
        )

        expected_key_names = [
            "spherical_harmonics_l",
            "species_center",
            "species_neighbor",
        ]
        assert spherical_expansion_1.keys.names == expected_key_names
        assert spherical_expansion_1.property_names == ["n"]

        # Fill blocks with `species_neighbor` from ALL blocks. If we don't do this
        # merging blocks along the ``sample`` direction might be not possible.
        keys_to_move = Labels(
            names="species_neighbor",
            values=np.unique(spherical_expansion_1.keys["species_neighbor"]).reshape(
                -1, 1
            ),
        )

        spherical_expansion_1 = spherical_expansion_1.keys_to_properties(keys_to_move)

        if self.calculator_2 is None:
            spherical_expansion_2 = spherical_expansion_1
        else:
            spherical_expansion_2 = self.calculator_2.compute(
                systems=systems,
                gradients=gradients,
                use_native_system=use_native_system,
            )
            assert spherical_expansion_2.keys.names == expected_key_names
            assert spherical_expansion_2.property_names == ["n"]

            keys_to_move = Labels(
                names="species_neighbor",
                values=np.unique(
                    spherical_expansion_2.keys["species_neighbor"]
                ).reshape(-1, 1),
            )

            spherical_expansion_2 = spherical_expansion_2.keys_to_properties(
                keys_to_move
            )

        blocks = []
        keys = []

        for (ell, species_center), block_1 in spherical_expansion_1.items():
            factor = 1 / sqrt(2 * ell + 1)
            # Find that block indices that have the same spherical_harmonics_l and
            # species_center
            blocks_2 = spherical_expansion_2.blocks(
                spherical_harmonics_l=ell, species_center=species_center
            )
            for block_2 in blocks_2:
                # Make sure that samples are the same. This should not happen.
                assert block_1.samples == block_2.samples

                properties = Labels(
                    names=["species_neighbor_1", "n1", "species_neighbor_2", "n2"],
                    values=np.array(
                        [
                            properties_1.tolist() + properties_2.tolist()
                            for properties_1 in block_1.properties.values
                            for properties_2 in block_2.properties.values
                        ],
                        dtype=np.int32,
                    ),
                )

                # Compute the invariants by summation and store the results this is
                # equivalent to an einsum with: ima, imb -> iab
                data = factor * np.matmul(block_1.values.swapaxes(1, 2), block_2.values)

                new_block = TensorBlock(
                    values=data.reshape(data.shape[0], -1),
                    samples=block_1.samples,
                    components=[],
                    properties=properties,
                )

                for parameter in block_1.gradients_list():
                    if parameter == "positions":
                        _positions_gradients(new_block, block_1, block_2, factor)

                keys.append((ell, species_center))
                blocks.append(new_block)

        keys = Labels(
            names=["l", "species_center"],
            values=np.array(keys, dtype=np.int32),
        )

        return TensorMap(keys, blocks).keys_to_properties("l")


def _positions_gradients(new_block, block_1, block_2, factor):
    gradient_1 = block_1.gradient("positions")
    gradient_2 = block_2.gradient("positions")

    if len(gradient_1.samples) == 0 or len(gradient_2.samples) == 0:
        gradients_samples = Labels.empty(names=["sample", "structure", "atom"])
        gradient_values = np.array([]).reshape(0, 1, len(new_block.properties))
    else:
        # The "sample" dimension in the power spectrum gradient samples do
        # not necessarily matches the "sample" dimension in the spherical
        # expansion gradient samples. We create new samples by creating a
        # union between the two gradient samples.
        (
            gradients_samples,
            grad1_sample_idxs,
            grad2_sample_idxs,
        ) = gradient_1.samples.union_and_mapping(gradient_2.samples)

        gradient_values = np.zeros(
            [gradients_samples.values.shape[0], 3, len(new_block.properties)]
        )

        # this is equivalent to an einsum with: ixma, imb -> ixab
        gradient_1_values = factor * np.matmul(
            gradient_1.values.swapaxes(2, 3),
            block_2.values[gradient_1.samples["sample"], np.newaxis],
        )

        gradient_values[grad1_sample_idxs] += gradient_1_values.reshape(
            gradient_1.samples.values.shape[0], 3, -1
        )

        # this is equivalent to an einsum with: ima, ixmb -> ixab
        gradient_values_2 = factor * np.matmul(
            block_1.values[gradient_2.samples["sample"], np.newaxis].swapaxes(2, 3),
            gradient_2.values,
        )

        gradient_values[grad2_sample_idxs] += gradient_values_2.reshape(
            gradient_2.samples.values.shape[0], 3, -1
        )

    gradient = TensorBlock(
        values=gradient_values,
        samples=gradients_samples,
        components=[gradient_1.components[0]],
        properties=new_block.properties,
    )

    new_block.add_gradient("positions", gradient)
