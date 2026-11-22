//! Low-level FFI bindings to the FrankenPHP C shim + PHP embed SAPI.
//! Everything here is `unsafe` by design; safe wrappers live in `frankenphp-engine`.

#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(dead_code)]

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn shim_symbols_exist() {
        // compile-time link check: referencing the symbols is enough
        let _ = fp_startup as usize;
        let _ = fp_handle_request as usize;
        let _ = fp_version as usize;
    }
}
