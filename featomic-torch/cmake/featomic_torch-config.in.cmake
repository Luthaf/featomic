include(CMakeFindDependencyMacro)

# use the same version for featomic as the main CMakeLists.txt
set(REQUIRED_FEATOMIC_VERSION @REQUIRED_FEATOMIC_VERSION@)
find_package(featomic ${REQUIRED_FEATOMIC_VERSION} CONFIG REQUIRED)

# use the same version for metatensor_torch as the main CMakeLists.txt
set(REQUIRED_METATENSOR_TORCH_VERSION @REQUIRED_METATENSOR_TORCH_VERSION@)
find_package(metatensor_torch ${REQUIRED_METATENSOR_TORCH_VERSION} CONFIG REQUIRED)

# We can only load featomic_torch with the exact same version of Torch that
# was used to compile it (and is stored in BUILD_TORCH_VERSION)
set(BUILD_TORCH_VERSION @Torch_VERSION@)

find_package(Torch ${BUILD_TORCH_VERSION} REQUIRED EXACT)


include(${CMAKE_CURRENT_LIST_DIR}/featomic_torch-targets.cmake)
