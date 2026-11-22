//! Safe(ish) wrapper around the PHP embed engine. Batch-1 skeleton:
//! process-global lifecycle guard + single-script execution.

use std::ffi::{CStr, CString};
use std::ptr;
use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Debug)]
pub struct EngineError(String);

impl std::fmt::Display for EngineError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "engine error: {}", self.0)
    }
}
impl std::error::Error for EngineError {}

/// Global "engine started" guard — PHP embed can only start once per process.
static STARTED: AtomicBool = AtomicBool::new(false);

/// Start the PHP engine (idempotent). Must be called before `execute_script`.
pub fn startup() -> Result<(), EngineError> {
    let rc = unsafe { frankenphp_sys::fp_startup() };
    if rc != 0 {
        return Err(EngineError(format!("fp_startup failed: {rc}")));
    }
    STARTED.store(true, Ordering::SeqCst);
    Ok(())
}

/// Shut the engine down. After this, startup() may be called again.
pub fn shutdown() {
    if STARTED.swap(false, Ordering::SeqCst) {
        unsafe { frankenphp_sys::fp_shutdown() }
    }
}

pub fn version() -> &'static str {
    unsafe {
        CStr::from_ptr(frankenphp_sys::fp_version())
            .to_str()
            .unwrap_or("unknown")
    }
}

/// Execute one script file as a full request cycle, returning captured output.
pub fn execute_script(path: &str) -> Result<Vec<u8>, EngineError> {
    if !STARTED.load(Ordering::SeqCst) {
        return Err(EngineError("engine not started".into()));
    }
    let cpath = CString::new(path).map_err(|_| EngineError("nul byte in path".into()))?;
    let mut buf: *mut std::ffi::c_char = ptr::null_mut();
    let mut len: usize = 0;
    let rc = unsafe { frankenphp_sys::fp_handle_request(cpath.as_ptr(), &mut buf, &mut len) };
    if rc != 0 {
        return Err(EngineError(format!("request failed: {rc}")));
    }
    if buf.is_null() {
        return Ok(Vec::new());
    }
    let out = unsafe { std::slice::from_raw_parts(buf as *const u8, len).to_vec() };
    unsafe { frankenphp_sys::fp_free(buf as *mut std::ffi::c_void) };
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn startup_executes_hello_world_and_shuts_down() {
        // single test thread: PHP embed is process-global
        assert_eq!(startup().is_ok(), true);
        assert!(version().starts_with('8'));

        let dir = std::env::temp_dir().join("fp_engine_test");
        std::fs::create_dir_all(&dir).unwrap();
        let script = dir.join("hello.php");
        std::fs::write(&script, b"<?php echo 'Hello from Rust-owned FrankenPHP!';").unwrap();

        let body = execute_script(script.to_str().unwrap()).expect("exec");
        assert_eq!(body, b"Hello from Rust-owned FrankenPHP!");

        // second request proves request loop resets state cleanly
        let body2 = execute_script(script.to_str().unwrap()).expect("exec2");
        assert_eq!(body2, b"Hello from Rust-owned FrankenPHP!");

        shutdown();
        // after shutdown, execute must fail cleanly, not crash
        assert!(execute_script(script.to_str().unwrap()).is_err());
    }
}
