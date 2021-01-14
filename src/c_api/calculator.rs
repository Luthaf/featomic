use std::os::raw::c_char;
use std::ffi::CStr;
use std::ops::{Deref, DerefMut};

use crate::{Calculator, System};
use crate::descriptor::IndexValue;

use super::utils::copy_str_to_c;
use super::{catch_unwind, rascal_status_t};

use super::descriptor::rascal_descriptor_t;
use super::system::rascal_system_t;

/// Opaque type representing a Calculator
#[allow(non_camel_case_types)]
pub struct rascal_calculator_t(Calculator);

impl Deref for rascal_calculator_t {
    type Target = Calculator;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for rascal_calculator_t {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[no_mangle]
#[allow(clippy::module_name_repetitions)]
pub unsafe extern fn rascal_calculator(name: *const c_char, parameters: *const c_char) -> *mut rascal_calculator_t {
    let mut raw = std::ptr::null_mut();
    let unwind_wrapper = std::panic::AssertUnwindSafe(&mut raw);
    let status = catch_unwind(move || {
        check_pointers!(name, parameters);
        let name = CStr::from_ptr(name).to_str()?;
        let parameters = CStr::from_ptr(parameters).to_str()?;
        let calculator = Calculator::new(name, parameters.to_owned())?;
        let boxed = Box::new(rascal_calculator_t(calculator));

        *unwind_wrapper.0 = Box::into_raw(boxed);
        Ok(())
    });

    if status == rascal_status_t::RASCAL_SUCCESS {
        return raw;
    } else {
        return std::ptr::null_mut();
    }
}

#[no_mangle]
pub unsafe extern fn rascal_calculator_free(calculator: *mut rascal_calculator_t) -> rascal_status_t {
    catch_unwind(|| {
        if !calculator.is_null() {
            let boxed = Box::from_raw(calculator);
            std::mem::drop(boxed);
        }

        Ok(())
    })
}

#[no_mangle]
pub unsafe extern fn rascal_calculator_name(
    calculator: *const rascal_calculator_t,
    name: *mut c_char,
    bufflen: usize
) -> rascal_status_t {
    catch_unwind(|| {
        check_pointers!(calculator, name);
        copy_str_to_c(&(*calculator).name(), name, bufflen)?;
        Ok(())
    })
}

#[no_mangle]
pub unsafe extern fn rascal_calculator_parameters(
    calculator: *const rascal_calculator_t,
    parameters: *mut c_char,
    bufflen: usize
) -> rascal_status_t {
    catch_unwind(|| {
        check_pointers!(calculator, parameters);
        copy_str_to_c(&(*calculator).parameters(), parameters, bufflen)?;
        Ok(())
    })
}

#[no_mangle]
pub unsafe extern fn rascal_calculator_compute(
    calculator: *mut rascal_calculator_t,
    descriptor: *mut rascal_descriptor_t,
    systems: *mut rascal_system_t,
    count: usize
) -> rascal_status_t {
    catch_unwind(|| {
        if count == 0 {
            // TODO: warning
            return Ok(());
        }
        check_pointers!(calculator, descriptor, systems);

        // Create a Vec<&mut dyn System> from the passed systems
        let systems = std::slice::from_raw_parts_mut(systems, count);
        let mut references = Vec::new();
        for system in systems {
            references.push(system as &mut dyn System);
        }

        (*calculator).compute(&mut references, &mut *descriptor);

        Ok(())
    })
}

#[no_mangle]
pub unsafe extern fn rascal_calculator_compute_partial(
    calculator: *mut rascal_calculator_t,
    descriptor: *mut rascal_descriptor_t,
    systems: *mut rascal_system_t,
    systems_count: usize,
    samples: *const f64,
    samples_count: usize,
    features: *const f64,
    features_count: usize,
) -> rascal_status_t {
    catch_unwind(|| {
        if systems_count == 0 {
            // TODO: warning
            return Ok(());
        }
        check_pointers!(calculator, descriptor, systems);


        let samples = if samples.is_null() {
            None
        } else {
            let samples = samples as *const IndexValue;
            Some(std::slice::from_raw_parts(samples, samples_count))
        };

        let features = if features.is_null() {
            None
        } else {
            let features = features as *const IndexValue;
            Some(std::slice::from_raw_parts(features, features_count))
        };

        // Create a Vec<&mut dyn System> from the passed systems
        let systems = std::slice::from_raw_parts_mut(systems, systems_count);
        let mut references = Vec::new();
        for system in systems {
            references.push(system as &mut dyn System);
        }

        (*calculator).compute_partial_capi(&mut references, &mut *descriptor, samples, features)?;

        Ok(())
    })
}