r"""
.. _python-atomic-density:

Atomic Density
==============

the atomic density function :math:`g(r)`, often chosen to be a Gaussian or Delta
function, that defined the type of density under consideration. For a given central atom
:math:`i` in the system, the total density function :math:`\rho_i(\boldsymbol{r})`
around is then defined as :math:`\rho_i(\boldsymbol{r}) = \sum_{j} g(\boldsymbol{r} -
\boldsymbol{r}_{ij})`.

Atomic densities are represented as different child class of
:py:class:`rascaline.utils.AtomicDensityBase`: :py:class:`rascaline.utils.DeltaDensity`,
:py:class:`rascaline.utils.GaussianDensity`, and :py:class:`rascaline.utils.LodeDensity`
are provided, and you can implement your own by defining a new class.

.. autoclass:: rascaline.utils.AtomicDensityBase
    :members:
    :show-inheritance:

.. autoclass:: rascaline.utils.DeltaDensity
    :members:
    :show-inheritance:

.. autoclass:: rascaline.utils.GaussianDensity
    :members:
    :show-inheritance:

.. autoclass:: rascaline.utils.LodeDensity
    :members:
    :show-inheritance:

"""

from abc import ABC, abstractmethod
from typing import Union

import numpy as np


try:
    from scipy.special import gamma, gammainc

    HAS_SCIPY = True
except ImportError:
    HAS_SCIPY = False


class AtomicDensityBase(ABC):
    """Base class representing atomic densities."""

    @abstractmethod
    def compute(self, positions: Union[float, np.ndarray]) -> Union[float, np.ndarray]:
        """Compute the atomic density arising from atoms at ``positions``

        :param positions: positions to evaluate the atomic densities
        :returns: evaluated atomic density
        """


class DeltaDensity(AtomicDensityBase):
    r"""Delta atomic densities of the form :math:`g(r)=\delta(r)`."""

    def compute(self, positions: Union[float, np.ndarray]) -> Union[float, np.ndarray]:
        raise ValueError(
            "Compute function of the delta density should never called directly."
        )


class GaussianDensity(AtomicDensityBase):
    r"""Gaussian atomic density function.

    In rascaline, we use the convention

    .. math::

        g(r) = \frac{1}{(\pi \sigma^2)^{3/4}}e^{-\frac{r^2}{2\sigma^2}} \,.

    The prefactor was chosen such that the "L2-norm" of the Gaussian

    .. math::

            \|g\|^2 = \int \mathrm{d}^3\boldsymbol{r} |g(r)|^2 = 1\,,

    :param atomic_gaussian_width: Width of the atom-centered gaussian used to create the
        atomic density
    """

    def __init__(self, atomic_gaussian_width: float):
        self.atomic_gaussian_width = atomic_gaussian_width

    def compute(self, positions: Union[float, np.ndarray]) -> Union[float, np.ndarray]:
        atomic_gaussian_width_sq = self.atomic_gaussian_width**2
        return np.exp(-0.5 * positions**2 / atomic_gaussian_width_sq) / (
            np.pi * atomic_gaussian_width_sq
        ) ** (3 / 4)


class LodeDensity(AtomicDensityBase):
    r"""Smeared power law density, as used in LODE.

    It is defined as

    .. math::

        g(r) = \frac{1}{\Gamma\left(\frac{p}{2}\right)}
               \frac{\gamma\left( \frac{p}{2}, \frac{r^2}{2\sigma^2} \right)}
                    {r^p},

    where :math:`\Gamma(z)` is the Gamma function and :math:`\gamma(a, x)` is the
    incomplete lower Gamma function. However its evaluation at :math:`r=0` is
    problematic because :math:`g(r)` is of the form :math:`0/0`. For practical
    implementations, it is thus more convenient to rewrite the density as

    .. math::

        g(r) = \frac{1}{\Gamma(a)}\frac{1}{\left(2 \sigma^2\right)^a}
                \begin{cases}
                    \frac{1}{a} - \frac{x}{a+1} + \frac{x^2}{2(a+2)} + \mathcal{O}(x^3)
                        & x < 10^{-5} \\
                    \frac{\gamma(a,x)}{x^a}
                        & x \geq 10^{-5}
                \end{cases}

    It is convenient to use the expression for sufficiently small :math:`x` since the
    relative weight of the first neglected term is on the order of :math:`1/6x^3`.
    Therefore, the threshold :math:`x = 10^{-5}` leads to relative errors on the order
    of the machine epsilon.

    :param atomic_gaussian_width: Width of the atom-centered gaussian used to create the
        atomic density
    :param potential_exponent: Potential exponent of the decorated atom density.
        Currently only implemented for potential_exponent < 10. Some exponents can be
        connected to SOAP or physics-based quantities: p=0 uses Gaussian densities as in
        SOAP, p=1 uses 1/r Coulomb like densities, p=6 uses 1/r^6 dispersion like
        densities.
    """

    def __init__(self, atomic_gaussian_width: float, potential_exponent: int):
        if not HAS_SCIPY:
            raise ValueError("LodeDensity requires scipy to be installed")

        self.potential_exponent = potential_exponent
        self.atomic_gaussian_width = atomic_gaussian_width

    def _short_range(self, a, x):
        return 1 / a - x / (a + 1) + x**2 / (2 * (a + 2))

    def _long_range(self, a, x):
        return gamma(a) * gammainc(a, x) / x**a

    def compute(self, positions: Union[float, np.ndarray]) -> Union[float, np.ndarray]:
        if self.potential_exponent == 0:
            return GaussianDensity.compute(self, positions=positions)
        else:
            atomic_gaussian_width_sq = self.atomic_gaussian_width**2
            a = self.potential_exponent / 2
            x = positions**2 / (2 * atomic_gaussian_width_sq)

            prefac = 1 / gamma(a) / (2 * atomic_gaussian_width_sq) ** a

            return prefac * np.where(
                x < 1e-5,
                self._short_range(a, x),
                self._long_range(a, x),
            )
