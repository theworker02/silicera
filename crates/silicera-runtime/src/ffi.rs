//! Experimental C ABI surface for embedding Silicera-runtime in C/C++ hosts.
//!
//! Enabled with feature `c-abi`. All `unsafe` is isolated here. Rust callers
//! should prefer the safe `Dispatcher` API.
//!
//! # Symbols
//!
//! | Symbol | Role |
//! |--------|------|
//! | `silicera_init` | Process/library init (idempotent) |
//! | `silicera_profile_load` | Load `.hnep` into an opaque handle |
//! | `silicera_variant_select` | Size → variant name (NUL-terminated into caller buffer) |
//! | `silicera_profile_free` | Release handle |
//!
//! Status codes: `0` ok, negative = error (`-1` null, `-2` I/O/parse, `-3` buffer).

use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int, c_ulonglong};
use std::path::Path;
use std::ptr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

use crate::{Dispatcher, MismatchPolicy};

static INIT: AtomicBool = AtomicBool::new(false);

struct ProfileHandle {
    dispatcher: Dispatcher,
}

/// Opaque profile handle for C callers.
pub struct SiliceraProfile {
    inner: Mutex<ProfileHandle>,
}

/// Initialize the Silicera C ABI (idempotent). Returns `0`.
#[no_mangle]
pub extern "C" fn silicera_init() -> c_int {
    INIT.store(true, Ordering::SeqCst);
    0
}

/// Load an HNEP profile from a UTF-8 path.
///
/// On success writes a non-null `*out_profile` that must be freed with
/// [`silicera_profile_free`]. `strict_machine != 0` uses [`MismatchPolicy::StrictMachine`].
///
/// # Safety
///
/// `path` must be a valid NUL-terminated UTF-8 C string (or null → `-1`).
/// `out_profile` must be a valid writable pointer.
#[no_mangle]
pub unsafe extern "C" fn silicera_profile_load(
    path: *const c_char,
    strict_machine: c_int,
    out_profile: *mut *mut SiliceraProfile,
) -> c_int {
    if path.is_null() || out_profile.is_null() {
        return -1;
    }
    // SAFETY: caller guarantees NUL-terminated C string.
    let cstr = unsafe { CStr::from_ptr(path) };
    let path_str = match cstr.to_str() {
        Ok(s) => s,
        Err(_) => return -2,
    };
    let policy = if strict_machine != 0 {
        MismatchPolicy::StrictMachine
    } else {
        MismatchPolicy::FallbackBaseline
    };
    let dispatcher = match Dispatcher::open(Path::new(path_str), policy) {
        Ok(d) => d,
        Err(_) => return -2,
    };
    let boxed = Box::new(SiliceraProfile {
        inner: Mutex::new(ProfileHandle { dispatcher }),
    });
    // SAFETY: caller guarantees out_profile is writable.
    unsafe {
        *out_profile = Box::into_raw(boxed);
    }
    0
}

/// Select a variant name for `size_bytes` into `buf` (NUL-terminated).
///
/// Writes at most `buf_len` bytes including the trailing NUL.
///
/// # Safety
///
/// `profile` must be a handle from [`silicera_profile_load`] (or null → `-1`).
/// `buf` must point to a writable buffer of `buf_len` bytes when `buf_len > 0`.
#[no_mangle]
pub unsafe extern "C" fn silicera_variant_select(
    profile: *mut SiliceraProfile,
    size_bytes: c_ulonglong,
    buf: *mut c_char,
    buf_len: usize,
) -> c_int {
    if profile.is_null() || buf.is_null() || buf_len == 0 {
        return -1;
    }
    // SAFETY: handle from silicera_profile_load; not freed yet.
    let handle = unsafe { &*profile };
    let name = {
        let g = match handle.inner.lock() {
            Ok(g) => g,
            Err(_) => return -2,
        };
        g.dispatcher.size(size_bytes).to_string()
    };
    let cstr = match CString::new(name) {
        Ok(c) => c,
        Err(_) => return -2,
    };
    let bytes = cstr.as_bytes_with_nul();
    if bytes.len() > buf_len {
        return -3;
    }
    // SAFETY: buf has buf_len writable bytes.
    unsafe {
        ptr::copy_nonoverlapping(bytes.as_ptr() as *const c_char, buf, bytes.len());
    }
    0
}

/// Free a profile handle from [`silicera_profile_load`].
///
/// # Safety
///
/// `profile` must be null or a unique handle from `silicera_profile_load`.
/// Double-free is undefined.
#[no_mangle]
pub unsafe extern "C" fn silicera_profile_free(profile: *mut SiliceraProfile) {
    if profile.is_null() {
        return;
    }
    // SAFETY: unique ownership from Box::into_raw in load.
    unsafe {
        drop(Box::from_raw(profile));
    }
}
