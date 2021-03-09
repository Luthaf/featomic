#ifndef RASCALINE_H
#define RASCALINE_H

#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>

typedef enum rascal_indexes {
  RASCAL_INDEXES_FEATURES = 0,
  RASCAL_INDEXES_ENVIRONMENTS = 1,
  RASCAL_INDEXES_GRADIENTS = 2,
} rascal_indexes;

/*
 Status type returned by all functions in the C API.
 */
typedef enum rascal_status_t {
  /*
   The function succeeded
   */
  RASCAL_SUCCESS = 0,
  /*
   A function got an invalid parameter
   */
  RASCAL_INVALID_PARAMETER_ERROR = 1,
  /*
   There was an error reading or writing JSON
   */
  RASCAL_JSON_ERROR = 2,
  /*
   A string contains non-utf8 data
   */
  RASCAL_UTF8_ERROR = 3,
  /*
   There was an error of unknown kind
   */
  RASCAL_UNKNOWN_ERROR = 254,
  /*
   There was an internal error (rust panic)
   */
  RASCAL_INTERNAL_PANIC = 255,
} rascal_status_t;

/*
 Opaque type representing a Calculator
 */
typedef struct rascal_calculator_t rascal_calculator_t;

/*
 Opaque type representing a Descriptor
 */
typedef struct rascal_descriptor_t rascal_descriptor_t;

/*
 Pair of atoms coming from a neighbor list
 */
typedef struct rascal_pair_t {
  /*
   index of the first atom in the pair
   */
  uintptr_t first;
  /*
   index of the second atom in the pair
   */
  uintptr_t second;
  /*
   vector from the first atom to the second atom, wrapped inside the unit
   cell as required by periodic boundary conditions.
   */
  double vector[3];
} rascal_pair_t;

/*
 A `rascal_system_t` deals with the storage of atoms and related information,
 as well as the computation of neighbor lists.

 This struct contains a manual implementation of a virtual table, allowing to
 implement the rust `System` trait in C and other languages. Speaking in Rust
 terms, `user_data` contains a pointer (analog to `Box<Self>`) to the struct
 implementing the `System` trait; and then there is one function pointers
 (`Option<unsafe extern fn(XXX)>`) for each function in the `System` trait.

 A new implementation of the System trait can then be created in any language
 supporting a C API (meaning any language for our purposes); by correctly
 setting `user_data` to the actual data storage, and setting all function
 pointers to the correct functions. For an example of code doing this, see
 the `SystemBase` class in the Python interface to rascaline.
 */
typedef struct rascal_system_t {
  /*
   User-provided data should be stored here, it will be passed as the
   first parameter to all function pointers below.
   */
  void *user_data;
  /*
   This function should set `*size` to the number of atoms in this system
   */
  void (*size)(const void *user_data, uintptr_t *size);
  /*
   This function should set `*species` to a pointer to the first element of
   a contiguous array containing the atomic species. Each different atomic
   species should be identified with a different value. These values are
   usually the atomic number, but don't have to be.
   */
  void (*species)(const void *user_data, const uintptr_t **species);
  /*
   This function should set `*positions` to a pointer to the first element
   of a contiguous array containing the atomic cartesian coordinates.
   `positions[0], positions[1], positions[2]` must contain the x, y, z
   cartesian coordinates of the first atom, and so on.
   */
  void (*positions)(const void *user_data, const double **positions);
  /*
   This function should write the unit cell matrix in `cell`, which have
   space for 9 values.
   */
  void (*cell)(const void *user_data, double *cell);
  /*
   This function should compute the neighbor list with the given cutoff,
   and store it for later access using `pairs` or `pairs_containing`.
   */
  void (*compute_neighbors)(void *user_data, double cutoff);
  /*
   This function should set `*pairs` to a pointer to the first element of a
   contiguous array containing all pairs in this system; and `*count` to
   the size of the array/the number of pairs.

   This list of pair should only contain each pair once (and not twice as
   `i-j` and `j-i`), should not contain self pairs (`i-i`); and should only
   contains pairs where the distance between atoms is actually bellow the
   cutoff passed in the last call to `compute_neighbors`. This function is
   only valid to call after a call to `compute_neighbors`.
   */
  void (*pairs)(const void *user_data, const struct rascal_pair_t **pairs, uintptr_t *count);
  /*
   This function should set `*pairs` to a pointer to the first element of a
   contiguous array containing all pairs in this system containing the atom
   with index `center`; and `*count` to the size of the array/the number of
   pairs.

   The same restrictions on the list of pairs as `rascal_system_t::pairs`
   applies, with the additional condition that the pair `i-j` should be
   included both in the return of `pairs_containing(i)` and
   `pairs_containing(j)`.
   */
  void (*pairs_containing)(const void *user_data, uintptr_t center, const struct rascal_pair_t **pairs, uintptr_t *count);
} rascal_system_t;

typedef struct rascal_calculation_options_t {
  /*
   Copy the data from systems into native `SimpleSystem`. This can be
   faster than having to cross the FFI boundary too often.
   */
  bool use_native_system;
  /*
   List of samples on which to run the calculation. Use `NULL` to run the
   calculation on all samples.
   */
  const double *selected_samples;
  /*
   If selected_samples is not `NULL`, this should be set to the size of the
   selected_samples array
   */
  uintptr_t selected_samples_count;
  /*
   List of features on which to run the calculation. Use `NULL` to run the
   calculation on all features.
   */
  const double *selected_features;
  /*
   If selected_features is not `NULL`, this should be set to the size of the
   selected_features array
   */
  uintptr_t selected_features_count;
} rascal_calculation_options_t;

#ifdef __cplusplus
extern "C" {
#endif // __cplusplus

/*
 Get the last error message that was sent on the current thread
 */
const char *rascal_last_error(void);

struct rascal_descriptor_t *rascal_descriptor(void);

enum rascal_status_t rascal_descriptor_free(struct rascal_descriptor_t *descriptor);

enum rascal_status_t rascal_descriptor_values(const struct rascal_descriptor_t *descriptor,
                                              const double **data,
                                              uintptr_t *environments,
                                              uintptr_t *features);

enum rascal_status_t rascal_descriptor_gradients(const struct rascal_descriptor_t *descriptor,
                                                 const double **data,
                                                 uintptr_t *environments,
                                                 uintptr_t *features);

enum rascal_status_t rascal_descriptor_indexes(const struct rascal_descriptor_t *descriptor,
                                               enum rascal_indexes indexes,
                                               const double **values,
                                               uintptr_t *count,
                                               uintptr_t *size);

enum rascal_status_t rascal_descriptor_indexes_names(const struct rascal_descriptor_t *descriptor,
                                                     enum rascal_indexes indexes,
                                                     const char **names,
                                                     uintptr_t size);

enum rascal_status_t rascal_descriptor_densify(struct rascal_descriptor_t *descriptor,
                                               const char *const *variables,
                                               uintptr_t count);

struct rascal_calculator_t *rascal_calculator(const char *name, const char *parameters);

enum rascal_status_t rascal_calculator_free(struct rascal_calculator_t *calculator);

enum rascal_status_t rascal_calculator_name(const struct rascal_calculator_t *calculator,
                                            char *name,
                                            uintptr_t bufflen);

enum rascal_status_t rascal_calculator_parameters(const struct rascal_calculator_t *calculator,
                                                  char *parameters,
                                                  uintptr_t bufflen);

enum rascal_status_t rascal_calculator_compute(struct rascal_calculator_t *calculator,
                                               struct rascal_descriptor_t *descriptor,
                                               struct rascal_system_t *systems,
                                               uintptr_t systems_count,
                                               struct rascal_calculation_options_t options);

#ifdef __cplusplus
} // extern "C"
#endif // __cplusplus

#endif /* RASCALINE_H */
