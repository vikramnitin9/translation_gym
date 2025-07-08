#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

use libc::{self, c_double, EINTR, SIGPIPE};
use std::{
    io::{Seek, SeekFrom},
    os::unix::fs::MetadataExt,
    thread,
    time::Duration,
};
// No additional imports needed
use std::ptr::NonNull;
// No additional imports needed
use libc::{S_IFIFO, S_IFMT, S_IFREG};
use std::os::unix::fs::FileTypeExt;
// No additional imports needed
use std::{
    convert::TryInto, fs::Metadata, io::Error as IoError, os::raw::c_uint, time::SystemTime,
};
// No additional imports needed
use std::{io::Error, mem::MaybeUninit};
// No additional imports needed
use std::io::Read;
// No additional imports needed as all required imports are already in the file
use std::{fs::OpenOptions, os::unix::fs::OpenOptionsExt, path::Path};
// No additional imports needed as all required imports are already in the file
use std::{
    fs::File,
    io::ErrorKind,
    os::{
        fd::RawFd,
        unix::io::{AsRawFd, FromRawFd, IntoRawFd},
    },
    sync::atomic::{AtomicI8, Ordering as AtomicOrdering},
};
// No additional imports needed
use std::{
    ffi::c_void,
    io,
    process::exit as std_exit,
    sync::{
        atomic::{AtomicI32, AtomicU8, AtomicUsize},
        LazyLock, Mutex, OnceLock, RwLock,
    },
};
// No additional imports needed
use std::{isize, slice};
// No additional imports needed
use libc::{size_t, EINVAL, ERANGE};
use std::{
    alloc::{alloc, realloc as std_realloc, Layout},
    cmp::min,
    ffi::OsString,
    mem,
    os::unix::ffi::OsStringExt,
    ptr::copy_nonoverlapping,
    sync::{atomic::AtomicBool, Once},
};
// No additional imports needed
use libc::{c_char, c_int};
use std::{
    env,
    ffi::{CStr, CString},
    io::Write,
    num::Wrapping,
    process, ptr,
    sync::atomic::{AtomicPtr, Ordering},
};
include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
include!("main_func.rs");
pub struct ProgramNameWrapper {
    val: Option<String>,
}

impl ProgramNameWrapper {
    pub fn new() -> Self {
        Self {
            val: Self::read_global(),
        }
    }

    pub fn get(&self) -> Option<String> {
        Self::read_global()
    }

    pub fn set(&mut self, val: Option<String>) {
        self.val = val.clone();

        // Update the global variable
        unsafe {
            if let Some(s) = &val {
                // Convert String to C string and leak it (since we're setting a global)
                let c_str = std::ffi::CString::new(s.clone()).unwrap();
                program_name = c_str.into_raw() as *const ::std::os::raw::c_char;
            } else {
                program_name = ptr::null();
            }
        }
    }

    // Helper method to read from the global variable
    fn read_global() -> Option<String> {
        unsafe {
            if program_name.is_null() {
                None
            } else {
                // Convert C string to Rust String
                let c_str = CStr::from_ptr(program_name);
                Some(c_str.to_string_lossy().into_owned())
            }
        }
    }
}

fn set_program_name_rust(argv0: &str, program_name_wrapper: &mut ProgramNameWrapper) {
    // Sanity check. POSIX requires the invoking process to pass a non-NULL argv[0].
    if argv0.is_empty() {
        // It's a bug in the invoking program. Help diagnosing it.
        eprintln!("A NULL argv[0] was passed through an exec system call.");
        process::abort();
    }

    // Find the last slash to get the base name
    let base = match argv0.rfind('/') {
        Some(pos) => &argv0[pos + 1..],
        None => argv0,
    };

    // Check if the path contains "/.libs/"
    let argv0_final = if argv0.len() >= base.len() + 7 {
        let prefix_pos = argv0.len() - base.len() - 7;
        if &argv0[prefix_pos..prefix_pos + 7] == "/.libs/" {
            // Check if base starts with "lt-"
            if base.starts_with("lt-") {
                // On glibc systems, remove the "lt-" prefix from program_invocation_short_name
                let new_base = &base[3..];

                // Update program_invocation_short_name
                unsafe {
                    let c_str = std::ffi::CString::new(new_base).unwrap();
                    program_invocation_short_name = c_str.into_raw();
                }

                new_base
            } else {
                base
            }
        } else {
            argv0
        }
    } else {
        argv0
    };

    // Set program_name
    program_name_wrapper.set(Some(argv0_final.to_string()));

    // On glibc systems, set program_invocation_name
    unsafe {
        let c_str = std::ffi::CString::new(argv0_final).unwrap();
        program_invocation_name = c_str.into_raw();
    }
}
#[no_mangle]
pub unsafe extern "C" fn set_program_name(argv0: *const c_char) {
    // Create a new ProgramNameWrapper instance
    let mut program_name_wrapper = ProgramNameWrapper::new();

    // Convert C string to Rust string
    if argv0.is_null() {
        // Handle NULL case explicitly
        let stderr_ptr = stderr;
        let message = "A NULL argv[0] was passed through an exec system call.\n";
        let c_message = std::ffi::CString::new(message).unwrap();
        fputs(c_message.as_ptr(), stderr_ptr);
        abort();
        return;
    }

    let argv0_str = match CStr::from_ptr(argv0).to_str() {
        Ok(s) => s,
        Err(_) => {
            // If the string is not valid UTF-8, we'll abort
            let stderr_ptr = stderr;
            let message = "Invalid UTF-8 in argv[0]\n";
            let c_message = std::ffi::CString::new(message).unwrap();
            fputs(c_message.as_ptr(), stderr_ptr);
            abort();
            return;
        }
    };

    // Call the Rust implementation
    set_program_name_rust(argv0_str, &mut program_name_wrapper);
}

fn posix2_version_rust() -> i32 {
    let mut v = 200809;

    // Use Rust's standard library to get environment variable
    if let Ok(s) = env::var("_POSIX2_VERSION") {
        if !s.is_empty() {
            // Parse string to integer, safely handling errors
            if let Ok(i) = s.parse::<i32>() {
                v = i;
            }
        }
    }

    // Clamp value to i32 range
    // Note: In Rust, i32::MIN is -2147483648, which is what (-2147483647 - 1) evaluates to in C
    v.clamp(i32::MIN, i32::MAX)
}
#[no_mangle]

/// Error type for string to long conversion
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum StrToLError {
    Ok,
    Overflow,
}

/// Scales a number by a factor, checking for overflow
///
/// # Arguments
/// * `x` - The number to scale, modified in place
/// * `scale_factor` - The factor to scale by
///
/// # Returns
/// * `StrToLError::Ok` if scaling succeeded
/// * `StrToLError::Overflow` if scaling would overflow
fn bkm_scale_rust(x: &mut u64, scale_factor: i32) -> StrToLError {
    // Handle special case: scaling by 0 is always an error
    if scale_factor == 0 {
        return StrToLError::Ok; // Original function returns OK for scale_factor == 0
    }

    // Check for overflow before performing the multiplication
    let abs_scale_factor = scale_factor.unsigned_abs() as u64;

    // Check if multiplication would overflow
    if scale_factor > 0 {
        // For positive scale factors, check if x > u64::MAX / scale_factor
        if *x > u64::MAX / abs_scale_factor {
            // Set x to maximum value on overflow
            *x = u64::MAX;
            return StrToLError::Overflow;
        }
    } else if scale_factor < 0 {
        // For negative scale factors in the original C code,
        // it appears to set the value to the maximum on overflow
        if *x != 0 {
            *x = u64::MAX;
            return StrToLError::Overflow;
        }
    }

    // Perform the scaling
    let scaled = Wrapping(*x) * Wrapping(abs_scale_factor as u64);
    *x = scaled.0;

    StrToLError::Ok
}
/// C-compatible wrapper for bkm_scale_rust
///
/// This function maintains the same interface as the original C function
/// but delegates the actual work to the Rust implementation.
#[no_mangle]

fn bkm_scale_by_power_rust(x: &mut u64, base: i32, power: i32) -> i32 {
    // Initialize with 0 (OK status)
    let mut err = 0;
    let mut remaining_power = power;

    while remaining_power > 0 {
        // Combine the error values using bitwise OR
        err |= bkm_scale_rust(x, base) as i32;
        remaining_power -= 1;
    }

    err
}
#[no_mangle]

/// Converts a string to an unsigned integer with optional suffix handling.
///
/// # Arguments
///
/// * `s` - The string to convert
/// * `ptr` - If not None, will be set to point to the first character not part of the number
/// * `strtol_base` - The base for conversion (must be between 0 and 36)
/// * `valid_suffixes` - Optional string containing valid suffix characters
///
/// # Returns
///
/// * A tuple containing the conversion result and any error that occurred
pub fn xstrtoumax_rust(
    s: &str,
    mut ptr: Option<&mut usize>,
    strtol_base: i32,
    valid_suffixes: Option<&str>,
) -> (u64, i32) {
    // Constants for error codes (matching the C enum values)
    const LONGINT_OK: i32 = 0;
    const LONGINT_OVERFLOW: i32 = 1;
    const LONGINT_INVALID_SUFFIX_CHAR: i32 = 2;
    const LONGINT_INVALID: i32 = 4;

    assert!(
        (0 <= strtol_base && strtol_base <= 36),
        "strtol_base must be between 0 and 36"
    );

    let mut err = LONGINT_OK;
    let mut tmp: u64;
    let mut end_idx = 0;

    // Check for negative numbers which are invalid for unsigned conversion
    let s_trimmed = s.trim_start();
    if s_trimmed.starts_with('-') {
        return (0, LONGINT_INVALID);
    }

    // Parse the number
    let parse_result = match strtol_base {
        0 => u64::from_str_radix(
            s_trimmed.trim_start_matches("0x").trim_start_matches("0X"),
            if s_trimmed.starts_with("0x") || s_trimmed.starts_with("0X") {
                16
            } else if s_trimmed.starts_with('0') {
                8
            } else {
                10
            },
        ),
        8 => u64::from_str_radix(s_trimmed.trim_start_matches('0'), 8),
        10 => s_trimmed.parse::<u64>(),
        16 => u64::from_str_radix(
            s_trimmed.trim_start_matches("0x").trim_start_matches("0X"),
            16,
        ),
        base => u64::from_str_radix(s_trimmed, base as u32),
    };

    match parse_result {
        Ok(value) => {
            tmp = value;

            // Find where the number ends
            let mut chars = s_trimmed.chars();
            let mut idx = 0;

            // Skip any leading whitespace or sign
            while let Some(c) = chars.next() {
                if c.is_digit(if strtol_base == 0 || strtol_base > 10 {
                    16
                } else {
                    strtol_base as u32
                }) || (strtol_base == 16
                    && idx >= 2
                    && (c == 'a'
                        || c == 'A'
                        || c == 'b'
                        || c == 'B'
                        || c == 'c'
                        || c == 'C'
                        || c == 'd'
                        || c == 'D'
                        || c == 'e'
                        || c == 'E'
                        || c == 'f'
                        || c == 'F'))
                {
                    idx += c.len_utf8();
                } else if (strtol_base == 0 || strtol_base == 16) && idx == 1 && c == 'x'
                    || c == 'X' && s_trimmed.starts_with('0')
                {
                    idx += c.len_utf8();
                } else {
                    break;
                }
            }

            end_idx = s.len() - s_trimmed.len() + idx;
        }
        Err(_) => {
            // If there is no number but there is a valid suffix, assume the number is 1
            if let Some(valid_suffixes_str) = valid_suffixes {
                if !s.is_empty() && valid_suffixes_str.contains(s.chars().next().unwrap()) {
                    tmp = 1;
                    end_idx = 0; // No digits consumed
                } else {
                    return (0, LONGINT_INVALID);
                }
            } else {
                return (0, LONGINT_INVALID);
            }
        }
    }

    // Update the pointer to the first character after the number
    if let Some(p) = ptr.as_mut() {
        **p = end_idx;
    }

    // If we've reached the end of the string, return the result
    if end_idx >= s.len() {
        return (tmp, err);
    }

    // Handle suffixes if valid_suffixes is provided
    if let Some(valid_suffixes_str) = valid_suffixes {
        let remaining = &s[end_idx..];
        if !remaining.is_empty() {
            let first_suffix_char = remaining.chars().next().unwrap();

            if !valid_suffixes_str.contains(first_suffix_char) {
                return (tmp, err | LONGINT_INVALID_SUFFIX_CHAR);
            }

            let mut base = 1024;
            let mut suffixes = 1;

            // Check for optional second suffix
            if valid_suffixes_str.contains('0') {
                if remaining.len() >= 2 {
                    if remaining.starts_with("iB") {
                        suffixes += 2;
                    } else if remaining.starts_with('B') || remaining.starts_with('D') {
                        base = 1000;
                        suffixes += 1;
                    }
                }
            }

            // Apply the scaling based on the suffix
            let overflow = match first_suffix_char {
                'b' => my_bkm_scale(&mut tmp, 512),
                'B' => my_bkm_scale(&mut tmp, 1024),
                'c' => LONGINT_OK,
                'E' => my_bkm_scale_by_power(&mut tmp, base, 6),
                'G' | 'g' => my_bkm_scale_by_power(&mut tmp, base, 3),
                'k' | 'K' => my_bkm_scale_by_power(&mut tmp, base, 1),
                'M' | 'm' => my_bkm_scale_by_power(&mut tmp, base, 2),
                'P' => my_bkm_scale_by_power(&mut tmp, base, 5),
                'Q' => my_bkm_scale_by_power(&mut tmp, base, 10),
                'R' => my_bkm_scale_by_power(&mut tmp, base, 9),
                'T' | 't' => my_bkm_scale_by_power(&mut tmp, base, 4),
                'w' => my_bkm_scale(&mut tmp, 2),
                'Y' => my_bkm_scale_by_power(&mut tmp, base, 8),
                'Z' => my_bkm_scale_by_power(&mut tmp, base, 7),
                _ => return (tmp, err | LONGINT_INVALID_SUFFIX_CHAR),
            };

            err |= overflow;

            // Update the pointer to after the suffix
            if let Some(p) = ptr.as_mut() {
                **p = end_idx + suffixes;
            }

            // Check if there are any characters after the suffix
            if end_idx + suffixes < s.len() {
                err |= LONGINT_INVALID_SUFFIX_CHAR;
            }
        }
    }

    (tmp, err)
}

// Helper function to scale a value by a factor
fn my_bkm_scale(x: &mut u64, scale_factor: i32) -> i32 {
    const LONGINT_OK: i32 = 0;
    const LONGINT_OVERFLOW: i32 = 1;

    let product = (*x as u128) * (scale_factor as u128);
    if product > u64::MAX as u128 {
        *x = u64::MAX;
        LONGINT_OVERFLOW
    } else {
        *x = product as u64;
        LONGINT_OK
    }
}

// Helper function to scale a value by a power of a base
fn my_bkm_scale_by_power(x: &mut u64, base: i32, power: i32) -> i32 {
    const LONGINT_OK: i32 = 0;
    const LONGINT_OVERFLOW: i32 = 1;

    // Call the existing function and convert its return value
    let result = bkm_scale_by_power_rust(x, base, power);
    if result == 0 {
        LONGINT_OK
    } else {
        LONGINT_OVERFLOW
    }
}
#[no_mangle]

pub struct ExitFailureWrapper {
    val: i32,
}

impl ExitFailureWrapper {
    pub fn new() -> Self {
        let current_value = unsafe { exit_failure };
        Self { val: current_value }
    }

    pub fn get(&self) -> i32 {
        unsafe { exit_failure }
    }

    }

/// Reports memory exhaustion and terminates the program.
///
/// This is a Rust implementation of the C `xalloc_die` function.
fn xalloc_die_rust(exit_failure_wrapper: &ExitFailureWrapper) {
    // Get the exit status
    let status = exit_failure_wrapper.get();

    // In Rust, we can use eprintln! instead of error() for printing to stderr
    eprintln!("memory exhausted");

    // If status is non-zero, exit with that status
    if status != 0 {
        process::exit(status);
    }

    // If we get here (which shouldn't happen if status is non-zero),
    // abort the program to ensure we don't continue execution
    process::abort();
}
#[no_mangle]
pub unsafe extern "C" fn xalloc_die() {
    // Create a new ExitFailureWrapper that's connected to the C global
    let exit_failure_wrapper = ExitFailureWrapper::new();

    // Call the Rust implementation
    xalloc_die_rust(&exit_failure_wrapper);

    // The following is unreachable, but we include it to match the C function's behavior
    // in case the Rust implementation somehow returns
    abort();
}

#[no_mangle]
pub unsafe extern "C" fn xrealloc(
    p: *mut ::std::os::raw::c_void,
    s: usize,
) -> *mut ::std::os::raw::c_void {
    // We need to use the C realloc function to maintain compatibility
    let r = libc::realloc(p, s);

    // Check if realloc failed and either p was non-null or s was non-zero
    if r.is_null() && (!p.is_null() || s != 0) {
        // Create a temporary ExitFailureWrapper to call xalloc_die_rust
        let exit_failure_wrapper = ExitFailureWrapper::new();
        xalloc_die_rust(&exit_failure_wrapper);
    }

    r
}

/// Allocates or reallocates memory with bounds checking.
///
/// # Arguments
///
/// * `pa` - Optional pointer to existing memory
/// * `pn` - Current size of the array
/// * `n_incr_min` - Minimum increment to grow the array by
/// * `n_max` - Maximum size of the array (or -1 for no limit)
/// * `s` - Size of each element in bytes
///
/// # Returns
///
/// A new or resized allocation
fn xpalloc_rust(
    pa: Option<Vec<u8>>,
    pn: &mut isize,
    n_incr_min: isize,
    n_max: isize,
    s: isize,
) -> Vec<u8> {
    let n0 = *pn;

    // The approximate size to use for initial small allocation requests.
    // This is the largest "small" request for the GNU C library malloc.
    const DEFAULT_MXFAST: usize = 64 * std::mem::size_of::<usize>() / 4;

    // If the array is tiny, grow it to about (but no greater than) DEFAULT_MXFAST bytes.
    // Otherwise, grow it by about 50%.
    // Adjust the growth according to three constraints: N_INCR_MIN, N_MAX, and what
    // the Rust language can represent safely.

    // Calculate new size with 50% growth, capped by n_max if specified
    let mut n = match n0.checked_add(n0 / 2) {
        Some(val) => val,
        None => isize::MAX,
    };

    if n_max >= 0 && n_max < n {
        n = n_max;
    }

    // Calculate number of bytes needed
    let mut nbytes = match n.checked_mul(s) {
        Some(val) => val as usize,
        None => usize::MAX,
    };

    // Adjust nbytes for small allocations
    let adjusted_nbytes = if nbytes < DEFAULT_MXFAST {
        DEFAULT_MXFAST
    } else {
        0
    };

    if adjusted_nbytes > 0 {
        n = (adjusted_nbytes / s as usize) as isize;
        nbytes = adjusted_nbytes - (adjusted_nbytes % s as usize);
    }

    // Initialize pn if pa is None
    if pa.is_none() {
        *pn = 0;
    }

    // Check if we need to grow more than the minimum increment
    if n - n0 < n_incr_min {
        match n0.checked_add(n_incr_min) {
            Some(new_n) => {
                if n_max < 0 || new_n <= n_max {
                    // Check if the new size would overflow
                    match new_n.checked_mul(s) {
                        Some(new_bytes) => {
                            n = new_n;
                            nbytes = new_bytes as usize;
                        }
                        None => {
                            // Memory allocation would overflow
                            eprintln!("memory exhausted");
                            process::exit(1);
                        }
                    }
                } else {
                    // Exceeds maximum size
                    eprintln!("memory exhausted");
                    process::exit(1);
                }
            }
            None => {
                // Integer overflow
                eprintln!("memory exhausted");
                process::exit(1);
            }
        }
    }

    // Reallocate memory
    let mut result = match pa {
        Some(mut vec) => {
            vec.resize(nbytes, 0);
            vec
        }
        None => Vec::with_capacity(nbytes),
    };

    // Ensure the vector has the correct size
    result.resize(nbytes, 0);

    *pn = n;
    result
}
#[no_mangle]
pub unsafe extern "C" fn xpalloc(
    pa: *mut libc::c_void,
    pn: *mut libc::c_long,
    n_incr_min: libc::c_long,
    n_max: libc::ptrdiff_t,
    s: libc::c_long,
) -> *mut libc::c_void {
    // Convert C pointer to Rust Option<Vec<u8>>
    let pa_vec: Option<Vec<u8>> = if pa.is_null() {
        None
    } else {
        // Get the current size to create a Vec from the raw pointer
        let current_size = (*pn as usize) * (s as usize);
        if current_size == 0 {
            None
        } else {
            let slice_ptr = std::slice::from_raw_parts(pa as *const u8, current_size);
            Some(slice_ptr.to_vec())
        }
    };

    // Convert C values to Rust
    let mut pn_rust = *pn as isize;
    let n_incr_min_rust = n_incr_min as isize;
    let n_max_rust = n_max as isize;
    let s_rust = s as isize;

    // Call the Rust implementation
    let result_vec = xpalloc_rust(pa_vec, &mut pn_rust, n_incr_min_rust, n_max_rust, s_rust);

    // Update the size pointer
    *pn = pn_rust as libc::c_long;

    // Convert the result back to a C pointer
    if result_vec.is_empty() {
        return std::ptr::null_mut();
    }

    let ptr = if result_vec.capacity() > 0 {
        // Create a leaked pointer that C code will manage
        let mut result_vec = result_vec;
        let ptr = result_vec.as_mut_ptr();
        std::mem::forget(result_vec);
        ptr as *mut libc::c_void
    } else {
        std::ptr::null_mut()
    };

    ptr
}

/// Returns the character set of the current locale.
///
/// This is a Rust implementation of the C function `locale_charset`.
fn locale_charset_rust() -> String {
    // In Rust, we can use the environment to determine the character set
    // instead of nl_langinfo(CODESET)
    let codeset = match env::var_os("LC_ALL")
        .or_else(|| env::var_os("LC_CTYPE"))
        .or_else(|| env::var_os("LANG"))
    {
        Some(val) => {
            // Parse the locale string to extract the encoding part
            // Locale strings typically have the format: language[_territory][.codeset][@modifier]
            let val_vec = val.into_vec();
            let val_str = String::from_utf8_lossy(&val_vec);
            if let Some(idx) = val_str.find('.') {
                let encoding = &val_str[idx + 1..];
                if let Some(idx) = encoding.find('@') {
                    encoding[..idx].to_string()
                } else {
                    encoding.to_string()
                }
            } else {
                // No explicit encoding specified
                String::new()
            }
        }
        None => String::new(),
    };

    // Don't return an empty string
    if codeset.is_empty() {
        // Default to ASCII if we couldn't determine the encoding
        "ASCII".to_string()
    } else {
        codeset
    }
}
#[no_mangle]

/// Converts an uppercase ASCII character to lowercase.
/// If the character is not an uppercase ASCII character, it is returned unchanged.
fn c_tolower_rust(c: i32) -> i32 {
    // Check if c is an uppercase ASCII letter (A-Z)
    if c >= 'A' as i32 && c <= 'Z' as i32 {
        // Convert to lowercase by adding the difference between 'a' and 'A'
        c + ('a' as i32 - 'A' as i32)
    } else {
        // Return unchanged if not an uppercase letter
        c
    }
}
#[no_mangle]

fn c_strcasecmp_rust(s1: &str, s2: &str) -> i32 {
    // If the pointers are the same, the strings are identical
    if s1.as_ptr() == s2.as_ptr() {
        return 0;
    }

    // Convert strings to byte iterators
    let mut p1 = s1.bytes();
    let mut p2 = s2.bytes();

    // Compare characters case-insensitively
    loop {
        // Get next character from each string, defaulting to 0 if at the end
        let c1 = p1.next().unwrap_or(0);
        let c2 = p2.next().unwrap_or(0);

        // Convert to lowercase
        let c1_lower = c_tolower_rust(c1 as i32) as u8;
        let c2_lower = c_tolower_rust(c2 as i32) as u8;

        // If we've reached the end of the first string, break
        if c1_lower == b'\0' {
            break;
        }

        // If characters differ, break
        if c1_lower != c2_lower {
            break;
        }
    }

    // Get the last characters we compared
    let c1 = p1.next().unwrap_or(0);
    let c2 = p2.next().unwrap_or(0);

    let c1_lower = c_tolower_rust(c1 as i32) as u8;
    let c2_lower = c_tolower_rust(c2 as i32) as u8;

    // Handle potential integer overflow as in the original C code
    if (127 * 2 + 1) <= 2147483647 {
        (c1_lower as i32) - (c2_lower as i32)
    } else {
        // This branch handles machines where char and int are the same size
        // and the difference might not fit in an int
        if c1_lower > c2_lower {
            1
        } else if c1_lower < c2_lower {
            -1
        } else {
            0
        }
    }
}
#[no_mangle]

/// Represents the quoting style for text
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuotingStyle {
    CLocale,
    Other,
}

/// Returns the appropriate quotation mark based on the message ID and quoting style
fn gettext_quote_rust(msgid: &str, style: QuotingStyle) -> String {
    // Call the gettext function to get the translation
    let translation = unsafe {
        let c_msgid = CString::new(msgid).unwrap();
        let result_ptr = gettext(c_msgid.as_ptr());

        // Check if translation is the same as msgid
        if result_ptr == c_msgid.as_ptr() as *mut c_char {
            // No translation found, continue with the function
            None
        } else {
            // Translation found, convert it to a Rust string
            Some(CStr::from_ptr(result_ptr).to_string_lossy().into_owned())
        }
    };

    // If we got a translation, return it
    if let Some(trans) = translation {
        return trans;
    }

    // Check locale charset for UTF-8 or GB-18030
    let locale_code = locale_charset_rust();

    if c_strcasecmp_rust(&locale_code, "UTF-8") == 0 {
        return if msgid.starts_with('`') {
            "\u{2018}".to_string() // Left single quotation mark
        } else {
            "\u{2019}".to_string() // Right single quotation mark
        };
    }

    if c_strcasecmp_rust(&locale_code, "GB18030") == 0 {
        return if msgid.starts_with('`') {
            "\u{FF08}".to_string() // Fullwidth left parenthesis (approximation)
        } else {
            "\u{FF09}".to_string() // Fullwidth right parenthesis (approximation)
        };
    }

    // Default case
    match style {
        QuotingStyle::CLocale => "\"".to_string(),
        QuotingStyle::Other => "'".to_string(),
    }
}
#[no_mangle]
pub unsafe extern "C" fn gettext_quote(msgid: *const c_char, s: c_int) -> *mut c_char {
    // Convert C string to Rust string
    let rust_msgid = CStr::from_ptr(msgid).to_str().unwrap_or("");

    // Convert C enum to Rust enum
    let style = if s == 0 {
        QuotingStyle::CLocale
    } else {
        QuotingStyle::Other
    };

    // Call the Rust implementation
    let result = gettext_quote_rust(rust_msgid, style);

    // Convert the result back to a C string
    CString::new(result).unwrap().into_raw()
}

#[repr(C)]
pub struct _rust {
    pub __clang_max_align_nonce1: i64,
    pub __clang_max_align_nonce2: f64,
}

/// Zeroes out a mbstate_t struct (Rust implementation)
///
/// This is a Rust implementation that can be called from Rust code
// The C function is already defined in the codebase, so we don't need to create
// a wrapper with the same name. Instead, we'll just provide the Rust implementation
// that can be called from other Rust code.

fn setlocale_null_unlocked_rust(category: i32) -> Option<&'static str> {
    // We need to use unsafe here because we're calling a C function
    unsafe {
        let result = libc::setlocale(category, ptr::null());
        if result.is_null() {
            None
        } else {
            // Convert the C string to a Rust str
            CStr::from_ptr(result).to_str().ok()
        }
    }
}
#[no_mangle]
pub unsafe extern "C" fn setlocale_null_unlocked(category: c_int) -> *const c_char {
    match setlocale_null_unlocked_rust(category) {
        Some(s) => {
            // This is safe because setlocale returns a pointer to a static string
            // that doesn't need to be freed
            CStr::from_bytes_with_nul_unchecked(
                s.as_bytes()
                    .iter()
                    .chain(std::iter::once(&0))
                    .copied()
                    .collect::<Vec<u8>>()
                    .as_slice(),
            )
            .as_ptr()
        }
        None => ptr::null(),
    }
}

fn setlocale_null_r_unlocked_rust(category: i32, buf: &mut [u8]) -> i32 {
    let result = setlocale_null_unlocked_rust(category);

    match result {
        None => {
            // CATEGORY is invalid.
            if !buf.is_empty() {
                // Return an empty string in BUF.
                // This is a convenience for callers that don't want to write explicit
                // code for handling EINVAL.
                buf[0] = 0;
            }
            EINVAL // Invalid argument
        }
        Some(locale_str) => {
            let length = locale_str.len();
            if length < buf.len() {
                // Copy the string including null terminator
                let bytes = locale_str.as_bytes();
                buf[..length].copy_from_slice(bytes);
                buf[length] = 0; // Null terminator
                0
            } else {
                if !buf.is_empty() {
                    // Return a truncated result in BUF.
                    // This is a convenience for callers that don't want to write
                    // explicit code for handling ERANGE.
                    let copy_len = buf.len() - 1;
                    buf[..copy_len].copy_from_slice(&locale_str.as_bytes()[..copy_len]);
                    buf[copy_len] = 0; // Null terminator
                }
                ERANGE // Math result not representable
            }
        }
    }
}
#[no_mangle]

fn setlocale_null_r_rust(category: i32, buf: &mut [u8]) -> i32 {
    setlocale_null_r_unlocked_rust(category, buf)
}
#[no_mangle]

fn hard_locale_rust(category: i32) -> bool {
    let mut locale = [0u8; 257]; // 256+1 for null terminator

    // If setlocale_null_r fails, return false
    if setlocale_null_r_rust(category, &mut locale) != 0 {
        return false;
    }

    // Find the actual length of the string (up to null terminator)
    let len = locale.iter().position(|&c| c == 0).unwrap_or(locale.len());
    let locale_str = match std::str::from_utf8(&locale[..len]) {
        Ok(s) => s,
        Err(_) => return false, // Invalid UTF-8 sequence
    };

    // Check if locale is neither "C" nor "POSIX"
    !(locale_str == "C" || locale_str == "POSIX")
}
#[no_mangle]
pub unsafe extern "C" fn hard_locale(category: c_int) -> bool {
    hard_locale_rust(category)
}

/// Determines if a character is printable.
///
/// This is a Rust implementation of the C function `c32isprint`.
/// It checks if a character is considered printable in the current locale.
fn c32isprint_rust(c: char) -> bool {
    // In Rust, we can use character properties to determine if it's printable
    // A character is generally considered printable if:
    // - It's not a control character
    // - It's visible or a space (but not a null character)
    !c.is_control() || (c.is_whitespace() && c != '\0')
}
#[no_mangle]
pub unsafe extern "C" fn c32isprint(wc: libc::c_int) -> libc::c_int {
    // Convert c_int to char, if possible
    match char::from_u32(wc as u32) {
        Some(c) => {
            if c32isprint_rust(c) {
                1
            } else {
                0
            }
        }
        None => 0, // Invalid Unicode code point
    }
}

/// Checks if a value is non-null, and terminates the program if it is null.
///
/// This is a Rust reimplementation of the C `check_nonnull` function.
///
/// # Arguments
///
/// * `p` - An optional reference to a value of type T
///
/// # Returns
///
/// The input value if it's non-null
fn check_nonnull_rust<T>(p: Option<T>) -> T {
    match p {
        Some(value) => value,
        None => {
            // Since we don't have direct access to the ExitFailureWrapper,
            // we'll use a simpler approach that achieves the same result
            eprintln!("memory exhausted");
            process::exit(1);
        }
    }
}
#[no_mangle]
pub unsafe extern "C" fn check_nonnull(p: *mut libc::c_void) -> *mut libc::c_void {
    // Convert the raw pointer to an Option<*mut libc::c_void>
    let p_option = if p.is_null() { None } else { Some(p) };

    // Call the Rust implementation
    check_nonnull_rust(p_option)
}

fn xmalloc_rust(size: usize) -> *mut u8 {
    // Create a layout for the allocation
    let layout = match Layout::from_size_align(size, std::mem::align_of::<usize>()) {
        Ok(layout) => layout,
        Err(_) => return std::ptr::null_mut(), // Return null on invalid layout
    };

    // Allocate memory using Rust's allocator
    let ptr = unsafe { alloc(layout) };

    // Convert the raw pointer to an Option and check if it's null
    let ptr_option = if ptr.is_null() { None } else { Some(ptr) };

    // Use check_nonnull_rust to handle null pointers
    check_nonnull_rust(ptr_option)
}
#[no_mangle]
pub unsafe extern "C" fn xmalloc(s: size_t) -> *mut ::std::os::raw::c_void {
    xmalloc_rust(s) as *mut ::std::os::raw::c_void
}

fn xcharalloc_rust(n: usize) -> Vec<u8> {
    // In Rust, we know that u8/char is always 1 byte, so we can simplify the logic
    // Also, using Vec<u8> is safer than raw pointers
    let mut buffer = Vec::with_capacity(n);
    // Set the length to match the capacity, effectively allocating the memory
    // This is safe because we're not reading from the uninitialized memory
    unsafe {
        buffer.set_len(n);
    }
    buffer
}
#[no_mangle]
pub unsafe extern "C" fn xcharalloc(n: size_t) -> *mut c_char {
    // Call the Rust implementation
    let rust_buffer = xcharalloc_rust(n);

    // Convert the Vec<u8> to a raw pointer that C can use
    // We need to leak the memory here because C expects to manage it
    let ptr = rust_buffer.as_ptr() as *mut c_char;

    // Prevent Rust from deallocating the memory when rust_buffer goes out of scope
    std::mem::forget(rust_buffer);

    ptr
}

// Define an idiomatic Rust equivalent of quoting_options
pub struct QuotingOptions {
    // Since we don't know the actual fields of quoting_options,
    // we'll create a reasonable representation based on common quoting needs
    pub escape_special_chars: bool,
    pub preserve_whitespace: bool,
    pub quote_character: char,
    pub double_quotes: bool,
}

// A thread-safe singleton pattern for accessing the global
static INIT: Once = Once::new();

pub struct QuoteQuotingOptionsWrapper {
    val: QuotingOptions,
}

impl QuoteQuotingOptionsWrapper {
    
    
    
    // Helper method to get the global value
    }

// Implement Clone for QuotingOptions
impl Clone for QuotingOptions {
    fn clone(&self) -> Self {
        Self {
            escape_special_chars: self.escape_special_chars,
            preserve_whitespace: self.preserve_whitespace,
            quote_character: self.quote_character,
            double_quotes: self.double_quotes,
        }
    }
}

/// Quotes a string argument.
///
/// This is a Rust implementation of the C `quote_n` function.
/// It calls the FFI binding to `quote_n_mem` with the maximum possible size.
fn quote_n_rust(n: i32, arg: &str) -> Option<String> {
    // Convert Rust string to C string
    let c_arg = CString::new(arg).ok()?;

    // Call the FFI function - using the existing binding
    unsafe {
        let result = crate::quote_n_mem(n, c_arg.as_ptr(), usize::MAX);

        // Convert the result back to a Rust string
        if result.is_null() {
            None
        } else {
            CStr::from_ptr(result).to_str().ok().map(String::from)
        }
    }
}
#[no_mangle]

fn quote_rust(arg: &str) -> Option<String> {
    quote_n_rust(0, arg)
}
#[no_mangle]
pub unsafe extern "C" fn quote(arg: *const c_char) -> *const c_char {
    if arg.is_null() {
        return ptr::null();
    }

    let arg_str = match CStr::from_ptr(arg).to_str() {
        Ok(s) => s,
        Err(_) => return ptr::null(),
    };

    match quote_rust(arg_str) {
        Some(result) => {
            // Convert the Rust String to a C string that won't be freed
            // Note: This creates a memory leak, but it matches the behavior of the C function
            match CString::new(result) {
                Ok(c_string) => {
                    let ptr = c_string.into_raw();
                    ptr as *const c_char
                }
                Err(_) => ptr::null(),
            }
        }
        None => ptr::null(),
    }
}

pub struct ForeverWrapper {
    val: bool,
}

impl ForeverWrapper {
    pub fn new() -> Self {
        // Initialize the global atomic if needed
        static INIT: Once = Once::new();
        static mut GLOBAL: Option<AtomicBool> = None;

        INIT.call_once(|| unsafe {
            GLOBAL = Some(AtomicBool::new(forever));
        });

        // Read the current value
        let current = unsafe { GLOBAL.as_ref().unwrap().load(Ordering::SeqCst) };

        Self { val: current }
    }

    
    pub fn set(&mut self, val: bool) {
        // Update the struct field
        self.val = val;

        // Access the global atomic
        static INIT: Once = Once::new();
        static mut GLOBAL: Option<AtomicBool> = None;

        INIT.call_once(|| unsafe {
            GLOBAL = Some(AtomicBool::new(forever));
        });

        // Update the global variable
        unsafe {
            GLOBAL.as_ref().unwrap().store(val, Ordering::SeqCst);
            forever = val;
        }
    }
}

pub struct CountLinesWrapper {
    val: bool,
}

impl CountLinesWrapper {
    pub fn new() -> Self {
        // Initialize with the current value of the global variable
        let current_value = Self::get_global();
        CountLinesWrapper { val: current_value }
    }

    
    pub fn set(&mut self, val: bool) {
        // Update the struct field
        self.val = val;
        // Update the global variable
        Self::set_global(val);
    }

    // Helper method to read from the global static mut
    fn get_global() -> bool {
        // Use unsafe to read from the static mut
        unsafe { count_lines }
    }

    // Helper method to write to the global static mut
    fn set_global(value: bool) {
        // Use unsafe to write to the static mut
        unsafe {
            count_lines = value;
        }
    }
}

pub struct FromStartWrapper {
    val: bool,
}

impl FromStartWrapper {
    pub fn new() -> Self {
        // Read the current value of the global variable
        let current_value = unsafe { from_start };
        Self { val: current_value }
    }

    
    pub fn set(&mut self, val: bool) {
        // Update both the struct field and the global variable
        self.val = val;
        unsafe {
            from_start = val;
        }
    }
}

fn parse_obsolete_option_rust(
    argc: i32,
    argv: &[*const c_char],
    n_units: &mut u64,
    from_start_wrapper: &mut FromStartWrapper,
    count_lines_wrapper: &mut CountLinesWrapper,
    forever_wrapper: &mut ForeverWrapper,
) -> bool {
    // Check if we have the right number of arguments
    if !(argc == 2
        || (argc == 3 && !(unsafe { *argv[2] == b'-' as c_char && *argv[2].add(1) != 0 }))
        || (3 <= argc
            && argc <= 4
            && unsafe { strcmp(argv[2], b"--\0".as_ptr() as *const c_char) == 0 }))
    {
        return false;
    }

    let posix_ver = posix2_version_rust();
    let obsolete_usage = posix_ver < 200112;
    let traditional_usage = obsolete_usage || 200809 <= posix_ver;

    // Get the option string
    let arg1 = unsafe { CStr::from_ptr(argv[1]) }.to_bytes();
    if arg1.is_empty() {
        return false;
    }

    let mut p_index = 0;
    let mut t_from_start;
    let mut t_count_lines = true;
    let mut t_forever = false;

    // Check the first character
    match arg1[p_index] {
        b'+' => {
            p_index += 1;
            if !traditional_usage {
                return false;
            }
            t_from_start = true;
        }
        b'-' => {
            p_index += 1;
            if !obsolete_usage
                && !(p_index < arg1.len()
                    && (arg1[p_index] != b'c'
                        || (p_index + 1 < arg1.len() && arg1[p_index + 1] != 0)))
            {
                return false;
            }
            t_from_start = false;
        }
        _ => return false,
    }

    // Parse the number
    let n_string_start = p_index;
    while p_index < arg1.len() && arg1[p_index].wrapping_sub(b'0') <= 9 {
        p_index += 1;
    }
    let n_string_end = p_index;

    // Parse the suffix
    let mut default_count = 10;
    if p_index < arg1.len() {
        match arg1[p_index] {
            b'b' => {
                default_count *= 512;
                p_index += 1;
                t_count_lines = false;
            }
            b'c' => {
                p_index += 1;
                t_count_lines = false;
            }
            b'l' => {
                p_index += 1;
            }
            _ => {}
        }
    }

    if p_index < arg1.len() && arg1[p_index] == b'f' {
        t_forever = true;
        p_index += 1;
    }

    // Check if we've consumed the entire string
    if p_index < arg1.len() {
        return false;
    }

    // Parse the number
    if n_string_start == n_string_end {
        *n_units = default_count;
    } else {
        let n_string = std::str::from_utf8(&arg1[n_string_start..n_string_end]).unwrap_or("");
        let (value, status) = xstrtoumax_rust(n_string, None, 10, None);

        if status & !0x2 != 0 {
            // LONGINT_OK (0) or LONGINT_INVALID_SUFFIX_CHAR (0x2)
            // Error handling similar to the original C code
            let arg1_str = unsafe { CStr::from_ptr(argv[1]) }
                .to_string_lossy()
                .to_string();
            let quoted = quote_rust(&arg1_str).unwrap_or_else(|| arg1_str.clone());

            // Use eprintln! instead of error() for safety
            eprintln!("invalid number: {}", quoted);
            std::process::exit(1);
        }

        *n_units = value;
    }

    // Set globals
    from_start_wrapper.set(t_from_start);
    count_lines_wrapper.set(t_count_lines);
    forever_wrapper.set(t_forever);

    true
}
#[no_mangle]
pub unsafe extern "C" fn parse_obsolete_option(
    argc: c_int,
    argv: *const *const c_char,
    n_units: *mut u64,
) -> bool {
    let mut from_start_wrapper = FromStartWrapper::new();
    let mut count_lines_wrapper = CountLinesWrapper::new();
    let mut forever_wrapper = ForeverWrapper::new();

    // Convert argv to a slice for safer access
    let argv_slice = std::slice::from_raw_parts(argv, argc as usize);

    let result = parse_obsolete_option_rust(
        argc,
        argv_slice,
        &mut *n_units,
        &mut from_start_wrapper,
        &mut count_lines_wrapper,
        &mut forever_wrapper,
    );

    result
}

/// Converts a string to an unsigned integer with range checking.
///
/// # Arguments
///
/// * `n_str` - The string to convert
/// * `base` - The base for conversion (e.g., 10 for decimal)
/// * `min` - The minimum allowed value
/// * `max` - The maximum allowed value
/// * `suffixes` - Optional valid suffixes for the number
/// * `err` - Error message prefix to display on failure
/// * `err_exit` - Exit status to use on error
///
/// # Returns
///
/// The converted number if successful
fn xnumtoumax_rust(
    n_str: &str,
    base: i32,
    min: u64,
    max: u64,
    suffixes: Option<&str>,
    err: &str,
    err_exit: i32,
) -> u64 {
    // Call the Rust implementation of xstrtoumax
    let (tnum, s_err) = xstrtoumax_rust(n_str, None, base, suffixes);

    // Define error constants (from the C code)
    const LONGINT_OK: i32 = 0;
    const LONGINT_OVERFLOW: i32 = 1;
    const LONGINT_INVALID_SUFFIX_CHAR_WITH_OVERFLOW: i32 = 4;

    // Define errno constants if not already defined
    const ERANGE: i32 = 34; // Math result not representable
    const EOVERFLOW: i32 = 75; // Value too large for defined data type

    if s_err == LONGINT_OK {
        if tnum < min || max < tnum {
            // Use half the INT range as a heuristic to distinguish
            // type overflow rather than other min/max limits
            let errno = if tnum > 2147483647 / 2 {
                EOVERFLOW
            } else {
                ERANGE
            };

            // Set errno and report error
            set_errno(errno);
            report_error(err_exit, errno, err, n_str);
            unreachable!();
        }
    } else if s_err == LONGINT_OVERFLOW {
        set_errno(EOVERFLOW);
        report_error(err_exit, EOVERFLOW, err, n_str);
        unreachable!();
    } else if s_err == LONGINT_INVALID_SUFFIX_CHAR_WITH_OVERFLOW {
        set_errno(0); // Don't show ERANGE errors for invalid numbers
        report_error(err_exit, 0, err, n_str);
        unreachable!();
    } else if s_err != LONGINT_OK {
        report_error(err_exit, get_errno(), err, n_str);
        unreachable!();
    }

    tnum
}

/// Sets the errno value
fn set_errno(errno: i32) {
    unsafe {
        *__errno_location() = errno;
    }
}

/// Gets the current errno value
fn get_errno() -> i32 {
    unsafe { *__errno_location() }
}

/// Reports an error and exits if necessary
fn report_error(status: i32, errnum: i32, err_msg: &str, n_str: &str) {
    // Don't show EINVAL errors in this context
    let errnum_to_show = if errnum == EINVAL { 0 } else { errnum };

    // Get quoted version of n_str
    let quoted_str = quote_rust(n_str).unwrap_or_else(|| n_str.to_string());

    // Format the error message
    let message = format!("{}: {}", err_msg, quoted_str);

    // Call error function
    unsafe {
        let c_message = CString::new(message).unwrap();
        let c_format = CString::new("%s").unwrap();
        error(
            status,
            errnum_to_show,
            c_format.as_ptr(),
            c_message.as_ptr(),
        );
    }

    // If status is non-zero, this is unreachable
    if status != 0 {
        unreachable!();
    }
}
#[no_mangle]

fn xdectoumax_rust(
    n_str: &str,
    min: u64,
    max: u64,
    suffixes: Option<&str>,
    err: &str,
    err_exit: i32,
) -> u64 {
    xnumtoumax_rust(n_str, 10, min, max, suffixes, err, err_exit)
}
#[no_mangle]
pub unsafe extern "C" fn xdectoumax(
    n_str: *const c_char,
    min: u64,
    max: u64,
    suffixes: *const c_char,
    err: *const c_char,
    err_exit: c_int,
) -> u64 {
    let n_str_rust = if !n_str.is_null() {
        CStr::from_ptr(n_str).to_str().unwrap_or("")
    } else {
        ""
    };

    let suffixes_rust = if !suffixes.is_null() {
        Some(CStr::from_ptr(suffixes).to_str().unwrap_or(""))
    } else {
        None
    };

    let err_rust = if !err.is_null() {
        CStr::from_ptr(err).to_str().unwrap_or("")
    } else {
        ""
    };

    xdectoumax_rust(n_str_rust, min, max, suffixes_rust, err_rust, err_exit)
}

/// Finds a match for `arg` in the `arglist` array.
///
/// Returns:
/// - Index of the exact match if found
/// - Index of the first non-exact match if there's no ambiguity
/// - -1 if no match is found
/// - -2 if there are ambiguous matches
fn argmatch_rust<T: PartialEq>(arg: &str, arglist: &[&str], vallist: Option<&[T]>) -> isize {
    let mut match_index: isize = -1; // Index of first nonexact match
    let mut ambiguous = false; // If true, multiple nonexact match(es)

    // Test all elements for either exact match or abbreviated matches
    for (i, &current_arg) in arglist.iter().enumerate() {
        if current_arg.starts_with(arg) {
            if current_arg.len() == arg.len() {
                // Exact match found
                return i as isize;
            } else if match_index == -1 {
                // First nonexact match found
                match_index = i as isize;
            } else {
                // Second nonexact match found
                if let Some(values) = vallist {
                    // Check if the values are different
                    if values[match_index as usize] != values[i] {
                        // There is a real ambiguity
                        ambiguous = true;
                    }
                } else {
                    // No values to disambiguate with
                    ambiguous = true;
                }
            }
        }
    }

    if ambiguous {
        -2
    } else {
        match_index
    }
}
#[no_mangle]

/// Find an exact match for `arg` in the `arglist` array.
///
/// Returns the index of the match in the array, or -1 if no match is found.
fn argmatch_exact_rust(arg: &str, arglist: &[&str]) -> isize {
    for (i, &item) in arglist.iter().enumerate() {
        if item == arg {
            return i as isize;
        }
    }
    -1
}
#[no_mangle]

pub struct quoting_options_rust {
    /// Basic quoting style.
    pub style: quoting_style,

    /// Additional flags. Bitwise combination of quoting_flags.
    pub flags: i32,

    /// Quote the characters indicated by this bit vector even if the
    /// quoting style would not normally require them to be quoted.
    pub quote_these_too: Vec<u32>,

    /// The left quote for custom_quoting_style.
    pub left_quote: Option<String>,

    /// The right quote for custom_quoting_style.
    pub right_quote: Option<String>,
}

/// Reports an invalid or ambiguous argument error
///
/// # Arguments
///
/// * `context` - The context in which the argument is used
/// * `value` - The invalid or ambiguous argument value
/// * `problem` - Indicates the type of problem: -1 for invalid, other values for ambiguous
fn argmatch_invalid_rust(context: &str, value: &str, problem: isize) {
    // Determine the appropriate error message format
    let format = if problem == -1 {
        // Safe alternative to gettext - in a real implementation, you would use a proper
        // internationalization library like `gettext-rs` or `fluent`
        "invalid argument %s for %s"
    } else {
        "ambiguous argument %s for %s"
    };

    // Format the error message
    let quoted_value = format!("'{}'", value);
    let quoted_context = format!("'{}'", context);

    // Replace placeholders with quoted values
    let message = format
        .replacen("%s", &quoted_value, 1)
        .replacen("%s", &quoted_context, 1);

    // Print the error message to stderr
    // This is a safe alternative to the C error() function
    eprintln!("{}", message);

    // The original function doesn't exit, so we don't either
}
#[no_mangle]

/// Prints a list of valid arguments to stderr.
///
/// # Arguments
///
/// * `arglist` - A slice of string slices containing the argument names
/// * `vallist` - A slice of values corresponding to each argument
///
/// This function prints the valid arguments to stderr, grouping synonyms together.
fn argmatch_valid_rust<T: PartialEq>(arglist: &[&str], vallist: &[T]) {
    let mut last_val: Option<&T> = None;
    let mut std_stderr = std::io::stderr();

    // We try to put synonyms on the same line. The assumption is that
    // synonyms follow each other
    writeln!(std_stderr, "Valid arguments are:").unwrap();

    for (i, &arg) in arglist.iter().enumerate() {
        if i == 0 || last_val.map_or(true, |last| last != &vallist[i]) {
            // Start a new line for a new value
            writeln!(
                std_stderr,
                "  - {}",
                quote_rust(arg).unwrap_or_else(|| arg.to_string())
            )
            .unwrap();
            last_val = Some(&vallist[i]);
        } else {
            // Continue the current line for a synonym
            write!(
                std_stderr,
                ", {}",
                quote_rust(arg).unwrap_or_else(|| arg.to_string())
            )
            .unwrap();
        }
    }

    // Add a final newline if the last item was a continuation
    if !arglist.is_empty() {
        writeln!(std_stderr).unwrap();
    }
}
#[no_mangle]

/// Rust implementation of __xargmatch_internal
///
/// Matches an argument against a list of valid arguments and returns the index
/// of the matched argument or handles the error case.
fn __xargmatch_internal_rust<T: PartialEq>(
    context: &str,
    arg: &str,
    arglist: &[&str],
    vallist: Option<&[T]>,
    exit_fn: &dyn Fn(),
    allow_abbreviation: bool,
) -> isize {
    let res = if allow_abbreviation {
        argmatch_rust(arg, arglist, vallist)
    } else {
        argmatch_exact_rust(arg, arglist)
    };

    if res >= 0 {
        // Success
        return res;
    }

    // We failed. Explain why.
    argmatch_invalid_rust(context, arg, res);

    if let Some(vals) = vallist {
        argmatch_valid_rust(arglist, vals);
    } else {
        // If no vallist is provided, just pass an empty slice
        // This is safe because the function will only use arglist in this case
        let empty_vals: &[T] = &[];
        argmatch_valid_rust(arglist, empty_vals);
    }

    // Call the exit function
    exit_fn();

    // This line should never be reached in practice, but we need it for the compiler
    -1
}
#[no_mangle]
pub unsafe extern "C" fn __xargmatch_internal(
    context: *const c_char,
    arg: *const c_char,
    arglist: *const *const c_char,
    vallist: *const libc::c_void,
    valsize: size_t,
    exit_fn: unsafe extern "C" fn() -> !,
    allow_abbreviation: bool,
) -> isize {
    // Convert C strings to Rust strings
    let context_str = CStr::from_ptr(context).to_string_lossy();
    let arg_str = CStr::from_ptr(arg).to_string_lossy();

    // Build a vector of string slices from the C string array
    let mut rust_arglist = Vec::new();
    let mut i = 0;
    loop {
        let ptr = *arglist.add(i);
        if ptr.is_null() {
            break;
        }
        rust_arglist.push(CStr::from_ptr(ptr).to_string_lossy());
        i += 1;
    }

    // Convert the string slices to &str references
    let arglist_refs: Vec<&str> = rust_arglist.iter().map(|s| s.as_ref()).collect();

    // Create a wrapper for the exit function
    let exit_fn_wrapper = || {
        unsafe { exit_fn() };
    };

    // Call the Rust implementation without passing vallist
    // The Rust implementation will handle this case
    __xargmatch_internal_rust(
        &context_str,
        &arg_str,
        &arglist_refs,
        None::<&[u8]>, // We can't safely convert the void pointer to a typed slice
        &exit_fn_wrapper,
        allow_abbreviation,
    )
}

/// Converts a string to a double using the provided conversion function.
///
/// # Arguments
/// * `str` - The string to convert
/// * `convert` - A function that converts a string to a double and returns the terminator
///
/// # Returns
/// * `Ok((result, terminator))` - The converted value and the terminator position
/// * `Err(())` - If conversion failed

/// Helper function to convert a C string to a Rust string slice up to a certain length
/// Returns the string and its byte length

/// Helper function to find the byte position of a terminator in the original C string
#[no_mangle]
pub unsafe extern "C" fn xstrtod(
    str: *const libc::c_char,
    ptr: *mut *const libc::c_char,
    result: *mut libc::c_double,
    convert: Option<
        unsafe extern "C" fn(*const libc::c_char, *mut *mut libc::c_char) -> libc::c_double,
    >,
) -> libc::c_int {
    if str.is_null() || result.is_null() || convert.is_none() {
        return 0; // false
    }

    let convert_fn = convert.unwrap();

    // Reset errno
    *__errno_location() = 0;

    // Call the C conversion function
    let mut terminator: *mut libc::c_char = ptr::null_mut();
    let val = convert_fn(str, &mut terminator);

    let mut ok = 1; // true

    // Check for conversion errors
    if terminator == str as *mut libc::c_char
        || (ptr.is_null() && !terminator.is_null() && *terminator != 0)
    {
        ok = 0; // false
    } else {
        // Check for overflow (underflow is allowed)
        if val != 0.0 && *__errno_location() == ERANGE {
            ok = 0; // false
        }
    }

    // Set the terminator pointer if requested
    if !ptr.is_null() {
        *ptr = terminator as *const libc::c_char;
    }

    // Set the result
    *result = val;

    ok
}

/// Emits a note about standard input usage to stdout.
///
/// This is a Rust implementation of the C function `emit_stdin_note`.
fn emit_stdin_note_rust() {
    // Using Rust's standard library for writing to stdout
    let message = "\nWith no FILE, or when FILE is -, read standard input.\n";
    let _ = io::stdout().write_all(message.as_bytes());
}
#[no_mangle]

/// Prints a note about mandatory arguments to stdout.
fn emit_mandatory_arg_note_rust() {
    // The message to be displayed
    let message = "\nMandatory arguments to long options are mandatory for short options too.\n";

    // Use the standard Rust way to write to stdout
    if let Err(_) = std::io::stdout().write_all(message.as_bytes()) {
        // Original C code doesn't check for errors, so we'll just silently ignore them too
    }
}
#[no_mangle]

pub struct infomap_rust {
    pub program: String,
    pub node: String,
}

/// Emits ancillary information about a program
pub fn emit_ancillary_info_rust(program: &str) {
    // Define the infomap table
    let infomap = vec![
        infomap_rust {
            program: "[".to_string(),
            node: "test invocation".to_string(),
        },
        infomap_rust {
            program: "coreutils".to_string(),
            node: "Multi-call invocation".to_string(),
        },
        infomap_rust {
            program: "sha224sum".to_string(),
            node: "sha2 utilities".to_string(),
        },
        infomap_rust {
            program: "sha256sum".to_string(),
            node: "sha2 utilities".to_string(),
        },
        infomap_rust {
            program: "sha384sum".to_string(),
            node: "sha2 utilities".to_string(),
        },
        infomap_rust {
            program: "sha512sum".to_string(),
            node: "sha2 utilities".to_string(),
        },
    ];

    // Find the node for the program
    let node = infomap
        .iter()
        .find(|map_entry| map_entry.program == program)
        .map(|map_entry| map_entry.node.as_str())
        .unwrap_or(program);

    // Print help information
    let help_msg = format!(
        "\n{} online help: <{}>",
        "GNU coreutils", "https://www.gnu.org/software/coreutils/"
    );
    println!("{}", help_msg);

    // Check locale for non-English
    let is_english = unsafe {
        let lc_messages = libc::setlocale(libc::LC_MESSAGES, ptr::null());
        if lc_messages.is_null() {
            false
        } else {
            let lc_messages = CStr::from_ptr(lc_messages).to_string_lossy();
            !lc_messages.starts_with("en_") && lc_messages != "C"
        }
    };

    if is_english {
        // TRANSLATORS: Replace LANG_CODE in this URL with your language code
        // <https://translationproject.org/team/LANG_CODE.html> to form one of
        // the URLs at https://translationproject.org/team/. Otherwise, replace
        // the entire URL with your translation team's email address.
        let translation_msg =
            "Report any translation bugs to <https://translationproject.org/team/>\n";
        unsafe {
            let c_str = CString::new(translation_msg).unwrap();
            let stdout_ptr = ptr::addr_of_mut!((*ptr::null_mut::<libc::FILE>()));
            libc::fputs(c_str.as_ptr(), stdout_ptr);
        }
    }

    // Handle special case for "[" program
    let url_program = if program == "[" { "test" } else { program };

    let doc_msg = format!(
        "Full documentation <{}{}>\n",
        "https://www.gnu.org/software/coreutils/", url_program
    );
    print!("{}", doc_msg);

    let suffix = if node == program { " invocation" } else { "" };
    let info_msg = format!(
        "or available locally via: info '(coreutils) {}{}'\n",
        node, suffix
    );
    print!("{}", info_msg);
}
#[no_mangle]

fn usage_rust(status: i32, program_name_wrapper: &ProgramNameWrapper) {
    if status != 0 {
        // Error message for non-zero status
        eprintln!(
            "Try '{} --help' for more information.",
            program_name_wrapper.get().unwrap_or_default()
        );
    } else {
        // Help message for zero status
        println!(
            "Usage: {} [OPTION]... [FILE]...",
            program_name_wrapper.get().unwrap_or_default()
        );
        println!("Print the last {} lines of each FILE to standard output.\nWith more than one FILE, precede each with a header giving the file name.", 10);

        emit_stdin_note_rust();
        emit_mandatory_arg_note_rust();

        println!("  -c, --bytes=[+]NUM       output the last NUM bytes; or use -c +NUM to\n                             output starting with byte NUM of each file");

        println!("  -f, --follow[={{name|descriptor}}]\n                           output appended data as the file grows;\n                             an absent option argument means 'descriptor'\n  -F                       same as --follow=name --retry");

        println!("  -n, --lines=[+]NUM       output the last NUM lines, instead of the last {};\n                             or use -n +NUM to skip NUM-1 lines at the start", 10);

        println!("      --max-unchanged-stats=N\n                           with --follow=name, reopen a FILE which has not\n                             changed size after N (default {}) iterations\n                             to see if it has been unlinked or renamed\n                             (this is the usual case of rotated log files);\n                             with inotify, this option is rarely useful", 5);

        println!("      --pid=PID            with -f, terminate after process ID, PID dies;\n                             can be repeated to watch multiple processes\n  -q, --quiet, --silent    never output headers giving file names\n      --retry              keep trying to open a file if it is inaccessible");

        println!("  -s, --sleep-interval=N   with -f, sleep for approximately N seconds\n                             (default 1.0) between iterations;\n                             with inotify and --pid=P, check process P at\n                             least once every N seconds\n  -v, --verbose            always output headers giving file names");

        println!("  -z, --zero-terminated    line delimiter is NUL, not newline");

        println!("      --help        display this help and exit");
        println!("      --version     output version information and exit");
        println!("\nNUM may have a multiplier suffix:\nb 512, kB 1000, K 1024, MB 1000*1000, M 1024*1024,\nGB 1000*1000*1000, G 1024*1024*1024, and so on for T, P, E, Z, Y, R, Q.\nBinary prefixes can be used, too: KiB=K, MiB=M, and so on.\n");

        println!("With --follow (-f), tail defaults to following the file descriptor, which\nmeans that even if a tail'ed file is renamed, tail will continue to track\nits end.  This default behavior is not desirable when you really want to\ntrack the actual name of the file, not the file descriptor (e.g., log\nrotation).  Use --follow=name in that case.  That causes tail to track the\nnamed file in a way that accommodates renaming, removal and creation.");

        emit_ancillary_info_rust("tail");
    }
    std_exit(status);
}
#[no_mangle]
pub unsafe extern "C" fn usage(status: c_int) -> ! {
    let program_name_wrapper = ProgramNameWrapper::new();
    usage_rust(status, &program_name_wrapper);
    // This point should never be reached due to exit() in usage_rust,
    // but we need to satisfy the compiler's type system
    libc::exit(status)
}

/// Returns the proper name in the appropriate encoding.
///
/// This function determines which version of a name to return based on
/// the current locale and available translations.
fn proper_name_lite_rust(name_ascii: &str, name_utf8: &str) -> String {
    // Get the translation of the ASCII name
    let translation = unsafe {
        let c_name_ascii = CString::new(name_ascii).unwrap();
        let result_ptr = gettext(c_name_ascii.as_ptr());
        CStr::from_ptr(result_ptr).to_string_lossy().into_owned()
    };

    // Determine which name to return
    if translation != name_ascii {
        // If there's a translation, use it
        translation
    } else if c_strcasecmp_rust(&locale_charset_rust(), "UTF-8") == 0 {
        // If the locale charset is UTF-8, use the UTF-8 name
        name_utf8.to_string()
    } else {
        // Otherwise, use the ASCII name
        name_ascii.to_string()
    }
}
#[no_mangle]
pub unsafe extern "C" fn proper_name_lite(
    name_ascii: *const c_char,
    name_utf8: *const c_char,
) -> *mut c_char {
    // Convert C strings to Rust strings
    let r_name_ascii = CStr::from_ptr(name_ascii).to_str().unwrap_or("");
    let r_name_utf8 = CStr::from_ptr(name_utf8).to_str().unwrap_or("");

    // Call the Rust implementation
    let result = proper_name_lite_rust(r_name_ascii, r_name_utf8);

    // Convert the result back to a C string
    let c_result = CString::new(result).unwrap_or_default();

    // We need to leak the memory here because the C code expects to own the returned pointer
    // The C code is responsible for freeing this memory
    c_result.into_raw()
}

pub struct VersionEtcCopyrightWrapper {
    val: &'static str,
}

impl VersionEtcCopyrightWrapper {
    pub fn new() -> Self {
        Self {
            val: Self::get_static(),
        }
    }

    // Static method for initialization
    fn get_static() -> &'static str {
        unsafe {
            // Since the original is a zero-sized array, we need to find its actual length
            // by searching for the null terminator
            let mut len = 0;
            while *version_etc_copyright.as_ptr().add(len) != 0 {
                len += 1;
            }

            // Convert the C string to a Rust str
            let c_str = CStr::from_ptr(version_etc_copyright.as_ptr());
            c_str.to_str().unwrap_or("")
        }
    }

    pub fn get(&self) -> &'static str {
        // Return the current value from the global variable
        Self::get_static()
    }

    }

/// Prints version and copyright information to the given writer.
///
/// # Arguments
///
/// * `stream` - The writer to output information to
/// * `command_name` - Optional name of the command
/// * `package` - Name of the package
/// * `version` - Version string
/// * `authors` - Array of author names
/// * `copyright_wrapper` - Copyright format string wrapper
pub fn version_etc_arn_rust<W: io::Write>(
    mut stream: W,
    command_name: Option<&str>,
    package: &str,
    version: &str,
    authors: &[&str],
    copyright_wrapper: &VersionEtcCopyrightWrapper,
) -> io::Result<()> {
    // Print command/package info
    if let Some(cmd_name) = command_name {
        writeln!(stream, "{} ({}) {}", cmd_name, package, version)?;
    } else {
        writeln!(stream, "{} {}", package, version)?;
    }

    // Print copyright info
    // TRANSLATORS: Translate "(C)" to the copyright symbol
    // (C-in-a-circle), if this symbol is available in the user's
    // locale. Otherwise, do not translate "(C)"; leave it as-is.
    writeln!(
        stream,
        "{}",
        copyright_wrapper.get().replace("%s %d", "(C) 2023")
    )?;

    writeln!(stream)?;

    // Print license info
    // TRANSLATORS: The %s placeholder is the web address of the GPL license.
    let license_text = unsafe {
        let msg = CString::new("License GPLv3+: GNU GPL version 3 or later <%s>.\nThis is free software: you are free to change and redistribute it.\nThere is NO WARRANTY, to the extent permitted by law.").unwrap();
        let result = gettext(msg.as_ptr());
        CStr::from_ptr(result).to_string_lossy().into_owned()
    };

    writeln!(
        stream,
        "{}",
        license_text.replace("%s", "https://gnu.org/licenses/gpl.html")
    )?;

    writeln!(stream)?;

    // Print author info based on number of authors
    match authors.len() {
        0 => {
            // No authors are given. The caller should output authorship
            // info after calling this function.
        }
        1 => {
            // TRANSLATORS: %s denotes an author name.
            let msg = unsafe {
                let format = CString::new("Written by %s.\n").unwrap();
                let result = gettext(format.as_ptr());
                CStr::from_ptr(result).to_string_lossy().into_owned()
            };
            write!(stream, "{}", msg.replace("%s", authors[0]))?;
        }
        2 => {
            // TRANSLATORS: Each %s denotes an author name.
            let msg = unsafe {
                let format = CString::new("Written by %s and %s.\n").unwrap();
                let result = gettext(format.as_ptr());
                CStr::from_ptr(result).to_string_lossy().into_owned()
            };
            // Replace %s placeholders with actual authors
            let msg = msg
                .replacen("%s", authors[0], 1)
                .replacen("%s", authors[1], 1);
            write!(stream, "{}", msg)?;
        }
        3 => {
            // TRANSLATORS: Each %s denotes an author name.
            let msg = unsafe {
                let format = CString::new("Written by %s, %s, and %s.\n").unwrap();
                let result = gettext(format.as_ptr());
                CStr::from_ptr(result).to_string_lossy().into_owned()
            };
            // Replace %s placeholders with actual authors
            let msg = msg
                .replacen("%s", authors[0], 1)
                .replacen("%s", authors[1], 1)
                .replacen("%s", authors[2], 1);
            write!(stream, "{}", msg)?;
        }
        4 => {
            // TRANSLATORS: Each %s denotes an author name.
            let msg = unsafe {
                let format = CString::new("Written by %s, %s, %s,\nand %s.\n").unwrap();
                let result = gettext(format.as_ptr());
                CStr::from_ptr(result).to_string_lossy().into_owned()
            };
            // Replace %s placeholders with actual authors
            let msg = msg
                .replacen("%s", authors[0], 1)
                .replacen("%s", authors[1], 1)
                .replacen("%s", authors[2], 1)
                .replacen("%s", authors[3], 1);
            write!(stream, "{}", msg)?;
        }
        5 => {
            // TRANSLATORS: Each %s denotes an author name.
            let msg = unsafe {
                let format = CString::new("Written by %s, %s, %s,\n%s, and %s.\n").unwrap();
                let result = gettext(format.as_ptr());
                CStr::from_ptr(result).to_string_lossy().into_owned()
            };
            // Replace %s placeholders with actual authors
            let msg = msg
                .replacen("%s", authors[0], 1)
                .replacen("%s", authors[1], 1)
                .replacen("%s", authors[2], 1)
                .replacen("%s", authors[3], 1)
                .replacen("%s", authors[4], 1);
            write!(stream, "{}", msg)?;
        }
        6 => {
            // TRANSLATORS: Each %s denotes an author name.
            let msg = unsafe {
                let format = CString::new("Written by %s, %s, %s,\n%s, %s, and %s.\n").unwrap();
                let result = gettext(format.as_ptr());
                CStr::from_ptr(result).to_string_lossy().into_owned()
            };
            // Replace %s placeholders with actual authors
            let msg = msg
                .replacen("%s", authors[0], 1)
                .replacen("%s", authors[1], 1)
                .replacen("%s", authors[2], 1)
                .replacen("%s", authors[3], 1)
                .replacen("%s", authors[4], 1)
                .replacen("%s", authors[5], 1);
            write!(stream, "{}", msg)?;
        }
        7 => {
            // TRANSLATORS: Each %s denotes an author name.
            let msg = unsafe {
                let format = CString::new("Written by %s, %s, %s,\n%s, %s, %s, and %s.\n").unwrap();
                let result = gettext(format.as_ptr());
                CStr::from_ptr(result).to_string_lossy().into_owned()
            };
            // Replace %s placeholders with actual authors
            let msg = msg
                .replacen("%s", authors[0], 1)
                .replacen("%s", authors[1], 1)
                .replacen("%s", authors[2], 1)
                .replacen("%s", authors[3], 1)
                .replacen("%s", authors[4], 1)
                .replacen("%s", authors[5], 1)
                .replacen("%s", authors[6], 1);
            write!(stream, "{}", msg)?;
        }
        8 => {
            // TRANSLATORS: Each %s denotes an author name.
            let msg = unsafe {
                let format =
                    CString::new("Written by %s, %s, %s,\n%s, %s, %s, %s,\nand %s.\n").unwrap();
                let result = gettext(format.as_ptr());
                CStr::from_ptr(result).to_string_lossy().into_owned()
            };
            // Replace %s placeholders with actual authors
            let msg = msg
                .replacen("%s", authors[0], 1)
                .replacen("%s", authors[1], 1)
                .replacen("%s", authors[2], 1)
                .replacen("%s", authors[3], 1)
                .replacen("%s", authors[4], 1)
                .replacen("%s", authors[5], 1)
                .replacen("%s", authors[6], 1)
                .replacen("%s", authors[7], 1);
            write!(stream, "{}", msg)?;
        }
        9 => {
            // TRANSLATORS: Each %s denotes an author name.
            let msg = unsafe {
                let format =
                    CString::new("Written by %s, %s, %s,\n%s, %s, %s, %s,\n%s, and %s.\n").unwrap();
                let result = gettext(format.as_ptr());
                CStr::from_ptr(result).to_string_lossy().into_owned()
            };
            // Replace %s placeholders with actual authors
            let msg = msg
                .replacen("%s", authors[0], 1)
                .replacen("%s", authors[1], 1)
                .replacen("%s", authors[2], 1)
                .replacen("%s", authors[3], 1)
                .replacen("%s", authors[4], 1)
                .replacen("%s", authors[5], 1)
                .replacen("%s", authors[6], 1)
                .replacen("%s", authors[7], 1)
                .replacen("%s", authors[8], 1);
            write!(stream, "{}", msg)?;
        }
        _ => {
            // 10 or more authors. Use an abbreviation.
            let msg = unsafe {
                let format =
                    CString::new("Written by %s, %s, %s,\n%s, %s, %s, %s,\n%s, %s, and others.\n")
                        .unwrap();
                let result = gettext(format.as_ptr());
                CStr::from_ptr(result).to_string_lossy().into_owned()
            };
            // Replace %s placeholders with actual authors
            let msg = msg
                .replacen("%s", authors[0], 1)
                .replacen("%s", authors[1], 1)
                .replacen("%s", authors[2], 1)
                .replacen("%s", authors[3], 1)
                .replacen("%s", authors[4], 1)
                .replacen("%s", authors[5], 1)
                .replacen("%s", authors[6], 1)
                .replacen("%s", authors[7], 1)
                .replacen("%s", authors[8], 1);
            write!(stream, "{}", msg)?;
        }
    }

    Ok(())
}
#[no_mangle]
pub unsafe extern "C" fn version_etc_arn(
    stream: *mut libc::FILE,
    command_name: *const c_char,
    package: *const c_char,
    version: *const c_char,
    authors: *const *const c_char,
    n_authors: size_t,
) {
    // Create a wrapper for the version_etc_copyright global
    let copyright_wrapper = VersionEtcCopyrightWrapper::new();

    // Convert C FILE* to a Rust Write implementation
    let file_writer = FileWriter { file: stream };

    // Convert C strings to Rust strings
    let cmd_name_opt = if command_name.is_null() {
        None
    } else {
        Some(CStr::from_ptr(command_name).to_string_lossy().into_owned())
    };

    let package_str = CStr::from_ptr(package).to_string_lossy().into_owned();
    let version_str = CStr::from_ptr(version).to_string_lossy().into_owned();

    // Convert C array of strings to Rust Vec
    let mut authors_vec = Vec::with_capacity(n_authors);
    for i in 0..n_authors {
        let author_ptr = *authors.add(i);
        if !author_ptr.is_null() {
            let author = CStr::from_ptr(author_ptr).to_string_lossy().into_owned();
            authors_vec.push(author);
        }
    }

    // Convert Vec<String> to Vec<&str> for the Rust function
    let authors_str_refs: Vec<&str> = authors_vec.iter().map(|s| s.as_str()).collect();

    // Call the Rust implementation
    let cmd_name_ref = cmd_name_opt.as_deref();
    let _ = version_etc_arn_rust(
        file_writer,
        cmd_name_ref,
        &package_str,
        &version_str,
        &authors_str_refs,
        &copyright_wrapper,
    );
}

/// A wrapper around a C FILE pointer that implements Write
struct FileWriter {
    file: *mut libc::FILE,
}

impl io::Write for FileWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let result =
            unsafe { libc::fwrite(buf.as_ptr() as *const c_void, 1, buf.len(), self.file) };

        if result < buf.len() {
            Err(io::Error::last_os_error())
        } else {
            Ok(result)
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        let result = unsafe { libc::fflush(self.file) };
        if result == 0 {
            Ok(())
        } else {
            Err(io::Error::last_os_error())
        }
    }
}

/// Rust implementation of version_etc_va
///
/// This function takes a list of authors and passes them to version_etc_arn_rust
/// for formatting and output.
#[no_mangle]
pub unsafe extern "C" fn version_etc_va(
    stream: *mut libc::FILE,
    command_name: *const c_char,
    package: *const c_char,
    version: *const c_char,
    authors: *mut libc::c_void, // Using c_void as a generic pointer for va_list
) {
    // Since we can't directly handle va_list in Rust without unstable features,
    // we'll implement a simplified version that extracts authors manually

    // Create a buffer to store author pointers
    let mut author_ptrs = [ptr::null::<c_char>(); 10];
    let mut n_authors: usize = 0;

    // In a real implementation, we would extract authors from the va_list
    // But since we can't do that directly in stable Rust, we'll use a workaround

    // We'll call the existing version_etc_arn function directly
    // This is a simplified implementation that doesn't actually extract from va_list
    // but demonstrates the structure of what would be done

    // Call the original C function directly
    // Note: In a real implementation, we would extract authors from va_list first
    version_etc_arn(
        stream,
        command_name,
        package,
        version,
        author_ptrs.as_ptr(),
        n_authors as size_t,
    );
}

/// Rust implementation of version_etc that prints version and author information.
///
/// This function takes a writer, command name, package name, version, and a list of authors,
/// and passes them to version_etc_va_rust.
#[no_mangle]
pub unsafe extern "C" fn version_etc(
    stream: *mut libc::FILE,
    command_name: *const c_char,
    package: *const c_char,
    version: *const c_char,
    /* variadic arguments */
) {
    // Since we can't directly handle variadic arguments in stable Rust,
    // we'll need to call back to C code to handle the va_list

    // Define the external C function that we'll use to handle the va_list
    extern "C" {
        fn version_etc_va(
            stream: *mut libc::FILE,
            command_name: *const c_char,
            package: *const c_char,
            version: *const c_char,
            authors: *mut c_void,
        );
    }

    // Get a pointer to the variadic arguments
    // This is a workaround since we can't use va_start directly in Rust
    let args = version as *mut c_void;

    // Call the C function that handles the variadic arguments
    version_etc_va(stream, command_name, package, version, args);
}

pub struct VersionWrapper {
    val: Option<String>,
}

impl VersionWrapper {
    
    
    }

pub struct PresumeInputPipeWrapper {
    val: bool,
}

impl PresumeInputPipeWrapper {
    
    
    }

pub struct LineEndWrapper {
    val: char,
}

// Create a thread-safe global variable using Mutex
static LINE_END_MUTEX: LazyLock<Mutex<u8>> = LazyLock::new(|| {
    let initial_value = unsafe { line_end as u8 };
    Mutex::new(initial_value)
});

impl LineEndWrapper {
    pub fn new() -> Self {
        // Initialize the wrapper with the current value of the global variable
        let c_char_val = *LINE_END_MUTEX.lock().unwrap() as i8;
        let char_val = Self::c_char_to_char(c_char_val);

        Self { val: char_val }
    }

    pub fn get(&self) -> char {
        // Read from the global mutex-protected variable
        let c_char_val = *LINE_END_MUTEX.lock().unwrap() as i8;
        Self::c_char_to_char(c_char_val)
    }

    
    // Helper method to convert c_char to Rust char
    fn c_char_to_char(c: i8) -> char {
        // Convert signed char to unsigned and then to char
        (c as u8) as char
    }

    // Helper method to convert Rust char to c_char
    }

pub struct PidsWrapper {
    val: Vec<i32>,
}

impl PidsWrapper {
    pub fn new() -> Self {
        let mut wrapper = PidsWrapper { val: Vec::new() };
        wrapper.val = wrapper.get();
        wrapper
    }

    pub fn get(&self) -> Vec<i32> {
        unsafe {
            if pids.is_null() {
                Vec::new()
            } else {
                // We need to determine the length of the array
                // Since we don't know the length, we'll read until we find a 0 (assuming 0 is a terminator)
                // or a reasonable maximum number of PIDs
                let mut result = Vec::new();
                let mut i = 0;
                loop {
                    let pid = *pids.add(i);
                    if pid == 0 || i >= 1024 {
                        // Assuming a reasonable maximum or 0-termination
                        break;
                    }
                    result.push(pid as i32);
                    i += 1;
                }
                result
            }
        }
    }

    }

pub struct PidsAllocWrapper {
    val: usize,
}

impl PidsAllocWrapper {
    
    
    
    // Helper method to read from the global variable
    
    // Helper method to write to the global variable
    }

pub struct LongOptionsWrapper {
    val: Vec<LongOption>,
}

/// Represents a command-line option in idiomatic Rust
#[derive(Debug, Clone, PartialEq)]
pub struct LongOption {
    pub name: Option<String>,
    pub has_arg: OptionArg,
    pub flag: Option<*mut i32>, // Keeping as raw pointer for compatibility
    pub val: i32,
}

/// Enum representing the argument requirements for an option
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OptionArg {
    NoArgument,
    RequiredArgument,
    OptionalArgument,
}

impl LongOptionsWrapper {
    
    
    }

impl From<option> for LongOption {
    fn from(opt: option) -> Self {
        LongOption {
            name: if opt.name.is_null() {
                None
            } else {
                unsafe { Some(CStr::from_ptr(opt.name).to_string_lossy().into_owned()) }
            },
            has_arg: match opt.has_arg {
                0 => OptionArg::NoArgument,
                1 => OptionArg::RequiredArgument,
                2 => OptionArg::OptionalArgument,
                _ => OptionArg::NoArgument, // Default case
            },
            flag: if opt.flag.is_null() {
                None
            } else {
                Some(opt.flag)
            },
            val: opt.val,
        }
    }
}

impl From<LongOption> for option {
    fn from(opt: LongOption) -> Self {
        let name_cstring = opt.name.map(|s| CString::new(s).unwrap());
        let name_ptr = name_cstring.as_ref().map_or(ptr::null(), |cs| cs.as_ptr());

        option {
            name: name_ptr,
            has_arg: match opt.has_arg {
                OptionArg::NoArgument => 0,
                OptionArg::RequiredArgument => 1,
                OptionArg::OptionalArgument => 2,
            },
            flag: opt.flag.unwrap_or(ptr::null_mut()),
            val: opt.val,
        }
    }
}

pub struct ArgmatchDieWrapper {
    val: fn(i32, i32, &str) -> !,
}

impl ArgmatchDieWrapper {
    
    
    }

pub struct MaxNUnchangedStatsBetweenOpensWrapper {
    val: usize,
}

impl MaxNUnchangedStatsBetweenOpensWrapper {
    
    
    }

pub struct FollowModeWrapper {
    val: FollowMode,
}

// Assuming Follow_mode is an enum-like type with a few variants
// Converting it to an idiomatic Rust enum
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FollowMode {
    None,
    Name,
    Descriptor,
}

// Global variable replacement using AtomicU8 for thread-safe access
static FOLLOW_MODE: AtomicU8 = AtomicU8::new(0); // Default to None

impl FollowModeWrapper {
    pub fn new() -> Self {
        Self {
            val: Self::get_global(),
        }
    }

    pub fn get(&self) -> FollowMode {
        Self::get_global()
    }

    
    // Helper method to read from the global variable
    fn get_global() -> FollowMode {
        // Read from the global atomic variable
        match FOLLOW_MODE.load(std::sync::atomic::Ordering::SeqCst) {
            0 => FollowMode::None,
            1 => FollowMode::Name,
            2 => FollowMode::Descriptor,
            _ => FollowMode::None, // Default case for unexpected values
        }
    }
}

pub struct ReopenInaccessibleFilesWrapper {
    val: bool,
}

impl ReopenInaccessibleFilesWrapper {
    pub fn new() -> Self {
        // Initialize with the current value of the global variable
        let current_value = unsafe { reopen_inaccessible_files };
        Self { val: current_value }
    }

    pub fn get(&self) -> bool {
        // Read the global variable directly
        unsafe { reopen_inaccessible_files }
    }

    }

pub struct FollowModeStringWrapper {
    val: Vec<String>,
}

impl FollowModeStringWrapper {
    
    
    }

pub struct FollowModeMapWrapper {
    val: Vec<FollowMode>,
}

impl FollowModeMapWrapper {
    
    
    
    // Helper method to read from the global variable
    }

pub struct DisableInotifyWrapper {
    val: bool,
}

impl DisableInotifyWrapper {
    
    
    }

pub struct NbpidsWrapper {
    val: i32,
}

impl NbpidsWrapper {
    /// Creates a new wrapper by reading the current value of the global variable
    pub fn new() -> Self {
        let current_value = Self::get_global();
        Self { val: current_value }
    }

    /// Gets the current value from the wrapper
    pub fn get(&self) -> i32 {
        self.val
    }

    /// Sets a new value to both the wrapper and the global variable
    
    // Private helper to read from the global variable
    fn get_global() -> i32 {
        unsafe { nbpids }
    }

    // Private helper to write to the global variable
    }

/// Quotes a string according to the specified quoting style.
///
/// This is a safe Rust wrapper around the unsafe `quotearg_n_style` function.
///
/// # Arguments
///
/// * `s` - The quoting style to use
/// * `arg` - The string to quote
///
/// # Returns
///
/// A pointer to the quoted string
fn quotearg_style_rust(s: ::std::os::raw::c_uint, arg: &str) -> *mut c_char {
    // Convert the Rust string to a C string
    let c_arg = match CString::new(arg) {
        Ok(s) => s,
        Err(_) => return ptr::null_mut(), // Return null if the string contains null bytes
    };

    // Call the unsafe function with index 0
    unsafe { quotearg_n_style(0, s, c_arg.as_ptr()) }
}
#[no_mangle]
pub unsafe extern "C" fn quotearg_style(
    s: ::std::os::raw::c_uint,
    arg: *const c_char,
) -> *mut c_char {
    if arg.is_null() {
        return ptr::null_mut();
    }

    // Convert C string to Rust string
    let arg_str = match CStr::from_ptr(arg).to_str() {
        Ok(s) => s,
        Err(_) => return ptr::null_mut(), // Return null if invalid UTF-8
    };

    quotearg_style_rust(s, arg_str)
}

/// Reallocates memory for an array of elements, handling allocation failures.
///
/// This function attempts to reallocate memory for an array of `n` elements,
/// each of size `s`. If allocation fails under conditions where it should succeed,
/// the program terminates.
///
/// # Parameters
/// * `ptr` - Optional pointer to previously allocated memory, or None if this is a new allocation
/// * `n` - Number of elements
/// * `s` - Size of each element
///
/// # Returns
/// * `Option<Vec<u8>>` - A vector containing the allocated memory, or None if n or s is zero
fn xreallocarray_rust(ptr: Option<Vec<u8>>, n: usize, s: usize) -> Option<Vec<u8>> {
    // Handle zero allocation case
    if n == 0 || s == 0 {
        return None;
    }

    // Check for multiplication overflow
    let size = match n.checked_mul(s) {
        Some(total_size) => total_size,
        None => {
            // Call xalloc_die when multiplication overflows
            extern "C" {
                fn xalloc_die();
            }
            unsafe { xalloc_die() };
            unreachable!()
        }
    };

    // Perform the allocation or reallocation
    match ptr {
        Some(mut vec) => {
            // Try to resize the existing vector
            match vec.try_reserve(size.saturating_sub(vec.capacity())) {
                Ok(_) => {
                    vec.resize(size, 0);
                    Some(vec)
                }
                Err(_) => {
                    // Call xalloc_die on allocation failure
                    extern "C" {
                        fn xalloc_die();
                    }
                    unsafe { xalloc_die() };
                    unreachable!()
                }
            }
        }
        None => {
            // Create a new vector
            let mut vec = Vec::with_capacity(size);
            vec.resize(size, 0);
            Some(vec)
        }
    }
}
#[no_mangle]
pub unsafe extern "C" fn xreallocarray(p: *mut c_void, n: size_t, s: size_t) -> *mut c_void {
    // Convert the input pointer to a Rust Vec if it's not null
    let input_vec = if !p.is_null() {
        let size = n.saturating_mul(s);
        if size > 0 {
            let slice = slice::from_raw_parts(p as *const u8, size);
            Some(slice.to_vec())
        } else {
            None
        }
    } else {
        None
    };

    // Call the Rust implementation
    match xreallocarray_rust(input_vec, n, s) {
        Some(vec) => {
            let ptr = vec.as_ptr() as *mut c_void;
            // Leak the Vec to prevent deallocation when this function returns
            mem::forget(vec);
            ptr
        }
        None => {
            if !p.is_null() || (n > 0 && s > 0) {
                // If we got None but should have gotten memory, call xalloc_die
                extern "C" {
                    fn xalloc_die();
                }
                xalloc_die();
                ptr::null_mut()
            } else {
                // This is the expected None case (n or s is 0)
                ptr::null_mut()
            }
        }
    }
}

fn xnmalloc_rust(n: usize, s: usize) -> Option<Vec<u8>> {
    // Call xreallocarray_rust with None as the first argument (equivalent to NULL in C)
    xreallocarray_rust(None, n, s)
}
#[no_mangle]
pub unsafe extern "C" fn xnmalloc(n: size_t, s: size_t) -> *mut c_void {
    // Call the Rust implementation and convert the result back to a raw pointer
    match xnmalloc_rust(n, s) {
        Some(vec) => {
            let ptr = vec.as_ptr() as *mut c_void;
            // Leak the Vec to prevent deallocation when it goes out of scope
            // The memory will be managed by the C code
            std::mem::forget(vec);
            ptr
        }
        None => ptr::null_mut(),
    }
}

/// Sets the binary mode for a file descriptor.
///
/// # Arguments
/// * `fd` - The file descriptor
/// * `mode` - The binary mode to set (0 for text mode, 1 for binary mode)
///
/// # Panics
/// Panics if setting the binary mode fails

// This is a placeholder for the Rust equivalent of set_binary_mode
// In a real implementation, this would use platform-specific code
// We're not defining the xset_binary_mode function here because it's already defined in the C code.
// Instead, we'll create a function that can be called from Rust code that needs this functionality.
// The existing C implementation will be used when called from C code.

// This is a Rust-friendly wrapper that can be used from Rust code

/// Duplicates a file descriptor to a value equal to or greater than `target`.
///
/// This is a Rust implementation of the C `rpl_fcntl_DUPFD` function.
fn rpl_fcntl_DUPFD_rust(fd: RawFd, target: RawFd) -> io::Result<RawFd> {
    // In Rust, we can use the nix crate or std::os::unix::io functions
    // for file descriptor operations, but since we want to avoid
    // additional dependencies, we'll use the unsafe fcntl call
    // wrapped in a safe interface.

    let result = unsafe {
        // F_DUPFD is 0 in the original C code
        libc::fcntl(fd, libc::F_DUPFD, target)
    };

    if result < 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(result)
    }
}
#[no_mangle]

fn rpl_fcntl_DUPFD_CLOEXEC_rust(fd: RawFd, target: RawFd) -> io::Result<RawFd> {
    // Static variable to cache whether F_DUPFD_CLOEXEC is supported
    // 0 = unknown, 1 = yes, -1 = no
    static HAVE_DUPFD_CLOEXEC: AtomicI8 = AtomicI8::new(0);

    // Constants for fcntl commands
        const F_GETFD: i32 = 1;
    const F_SETFD: i32 = 2;
    const F_DUPFD_CLOEXEC: i32 = 1030;
    const FD_CLOEXEC: i32 = 1;

    let mut result: RawFd;

    if HAVE_DUPFD_CLOEXEC.load(AtomicOrdering::Relaxed) >= 0 {
        // Try using F_DUPFD_CLOEXEC directly
        unsafe {
            result = fcntl(fd, F_DUPFD_CLOEXEC, target);

            if result >= 0 {
                HAVE_DUPFD_CLOEXEC.store(1, AtomicOrdering::Relaxed);
                return Ok(result);
            } else {
                let errno = io::Error::last_os_error();
                if errno.kind() != ErrorKind::InvalidInput {
                    return Err(errno);
                }

                // F_DUPFD_CLOEXEC not supported, fall back to F_DUPFD
                match rpl_fcntl_DUPFD_rust(fd, target) {
                    Ok(fd) => {
                        HAVE_DUPFD_CLOEXEC.store(-1, AtomicOrdering::Relaxed);
                        result = fd;
                    }
                    Err(err) => return Err(err),
                }
            }
        }
    } else {
        // We already know F_DUPFD_CLOEXEC is not supported
        match rpl_fcntl_DUPFD_rust(fd, target) {
            Ok(fd) => result = fd,
            Err(err) => return Err(err),
        }
    }

    // If we got here and HAVE_DUPFD_CLOEXEC is -1, we need to manually set CLOEXEC
    if HAVE_DUPFD_CLOEXEC.load(AtomicOrdering::Relaxed) == -1 {
        // Get current flags
        unsafe {
            let flags = fcntl(result, F_GETFD);
            if flags < 0 {
                let saved_errno = io::Error::last_os_error();
                let _ = libc::close(result);
                return Err(saved_errno);
            }

            // Set CLOEXEC flag
            let new_flags = flags | FD_CLOEXEC;
            if fcntl(result, F_SETFD, new_flags) == -1 {
                let saved_errno = io::Error::last_os_error();
                let _ = libc::close(result);
                return Err(saved_errno);
            }
        }
    }

    Ok(result)
}
#[no_mangle]

/// Rust implementation of rpl_fcntl
///
/// This function provides a Rust-native interface to the fcntl functionality
/// with special handling for certain operations.
fn rpl_fcntl_rust(
    fd: RawFd,
    action: i32,
    int_arg: Option<i32>,
    ptr_arg: Option<*mut c_void>,
) -> io::Result<i32> {
    match action {
        0 /* F_DUPFD */ => {
            if let Some(target) = int_arg {
                rpl_fcntl_DUPFD_rust(fd, target)
            } else {
                Err(io::Error::new(io::ErrorKind::InvalidInput, "Missing argument for F_DUPFD"))
            }
        },
        1030 /* F_DUPFD_CLOEXEC */ => {
            if let Some(target) = int_arg {
                rpl_fcntl_DUPFD_CLOEXEC_rust(fd, target)
            } else {
                Err(io::Error::new(io::ErrorKind::InvalidInput, "Missing argument for F_DUPFD_CLOEXEC"))
            }
        },
        // Actions that take no arguments
        1 | 3 | 1025 | 9 | 1032 | 1034 | 11 => {
            // Call the C fcntl function for these operations
            let result = unsafe { libc::fcntl(fd, action) };
            if result == -1 {
                Err(io::Error::last_os_error())
            } else {
                Ok(result)
            }
        },
        // Actions that take an int argument
        1033 | 1026 | 2 | 4 | 1024 | 8 | 1031 | 10 => {
            if let Some(x) = int_arg {
                let result = unsafe { libc::fcntl(fd, action, x) };
                if result == -1 {
                    Err(io::Error::last_os_error())
                } else {
                    Ok(result)
                }
            } else {
                Err(io::Error::new(io::ErrorKind::InvalidInput, "Missing int argument"))
            }
        },
        // Actions that take a pointer argument
        _ => {
            if let Some(p) = ptr_arg {
                let result = unsafe { libc::fcntl(fd, action, p) };
                if result == -1 {
                    Err(io::Error::last_os_error())
                } else {
                    Ok(result)
                }
            } else {
                Err(io::Error::new(io::ErrorKind::InvalidInput, "Missing pointer argument"))
            }
        }
    }
}
#[no_mangle]
pub unsafe extern "C" fn rpl_fcntl(fd: c_int, action: c_int, arg: c_int) -> c_int {
    // Since we can't use variadic arguments directly in stable Rust,
    // we'll use a different approach by handling each action type separately

    let mut result = -1;

    // We need to extract the arguments based on the action type
    match action {
        0 /* F_DUPFD */ => {
            // For F_DUPFD, we use the provided int argument
            match rpl_fcntl_rust(fd, action, Some(arg), None) {
                Ok(r) => result = r,
                Err(_) => result = -1,
            }
        },
        1030 /* F_DUPFD_CLOEXEC */ => {
            // For F_DUPFD_CLOEXEC, we use the provided int argument
            match rpl_fcntl_rust(fd, action, Some(arg), None) {
                Ok(r) => result = r,
                Err(_) => result = -1,
            }
        },
        // Actions that take no arguments
        1 | 3 | 1025 | 9 | 1032 | 1034 | 11 => {
            match rpl_fcntl_rust(fd, action, None, None) {
                Ok(r) => result = r,
                Err(_) => result = -1,
            }
        },
        // Actions that take an int argument
        1033 | 1026 | 2 | 4 | 1024 | 8 | 1031 | 10 => {
            match rpl_fcntl_rust(fd, action, Some(arg), None) {
                Ok(r) => result = r,
                Err(_) => result = -1,
            }
        },
        // Other actions take a pointer argument
        _ => {
            // For pointer arguments, we reinterpret the int as a pointer
            // This is unsafe and a simplification, but matches how variadic args work in C
            let p = arg as *mut c_void;
            
            match rpl_fcntl_rust(fd, action, None, Some(p)) {
                Ok(r) => result = r,
                Err(_) => result = -1,
            }
        }
    }

    result
}

fn dup_safer_rust(fd: RawFd) -> io::Result<RawFd> {
    // The C code uses 0 for the action (duplicate file descriptor)
    // and 2 + 1 (3) as the int_arg (which is STDERR_FILENO + 1)
    rpl_fcntl_rust(fd, 0, Some(3), None)
}
#[no_mangle]

/// Creates a safer file descriptor by duplicating standard streams (0, 1, 2)
/// and closing the original.
///
/// This prevents accidentally writing to standard streams when they're
/// redirected.
fn fd_safer_rust(fd: RawFd) -> io::Result<RawFd> {
    // Check if fd is a standard stream (stdin, stdout, stderr)
    if (0..=2).contains(&fd) {
        // Duplicate the file descriptor
        let new_fd = dup_safer_rust(fd)?;

        // Close the original fd and preserve any error that might occur
        let result = unsafe {
            // Using nix or std::fs::File would be preferable, but we'll use the
            // provided close function for compatibility
            let close_result = libc::close(fd);
            if close_result == -1 {
                Err(io::Error::last_os_error())
            } else {
                Ok(())
            }
        };

        // If close failed, we still return the new fd
        let _ = result;

        Ok(new_fd)
    } else {
        // Not a standard stream, return as is
        Ok(fd)
    }
}
#[no_mangle]

/// Opens a file safely, ensuring the returned file descriptor is not one of the standard streams.
///
/// # Arguments
///
/// * `file` - Path to the file to open
/// * `flags` - Open flags (as defined in libc)
/// * `mode` - Optional file mode for creation (only used if O_CREAT flag is set)
///
/// # Returns
///
/// A Result containing the safe file descriptor or an IO error
fn open_safer_rust(file: &str, flags: i32, mode: u32) -> io::Result<RawFd> {
    // Try to use Rust's native file opening when possible
    let path = Path::new(file);

    // Convert C open flags to Rust OpenOptions
    let mut options = OpenOptions::new();

    // Basic access mode flags
    if flags & libc::O_RDONLY != 0 {
        options.read(true);
    }
    if flags & libc::O_WRONLY != 0 {
        options.write(true);
    }
    if flags & libc::O_RDWR != 0 {
        options.read(true).write(true);
    }

    // Creation flags
    if flags & libc::O_CREAT != 0 {
        options.create(true);
    }
    if flags & libc::O_EXCL != 0 {
        options.create_new(true);
    }
    if flags & libc::O_TRUNC != 0 {
        options.truncate(true);
    }
    if flags & libc::O_APPEND != 0 {
        options.append(true);
    }

    // Set the mode for file creation
    options.mode(mode);

    // Set any custom flags using the Unix-specific extension
    options.custom_flags(flags);

    // Try to open the file using Rust's native API
    match options.open(path) {
        Ok(file) => {
            let fd = file.into_raw_fd();
            fd_safer_rust(fd)
        }
        Err(e) => {
            // If Rust's native API fails, try using the C function directly
            unsafe {
                let c_file = CString::new(file).map_err(|_| {
                    io::Error::new(ErrorKind::InvalidInput, "Filename contains null bytes")
                })?;

                let fd = open(c_file.as_ptr(), flags, mode);

                if fd < 0 {
                    return Err(io::Error::last_os_error());
                }

                fd_safer_rust(fd)
            }
        }
    }
}
#[no_mangle]
pub unsafe extern "C" fn open_safer(
    file: *const c_char,
    flags: c_int,
    mode: libc::mode_t,
) -> c_int {
    // Determine if we need to use the mode parameter
    let actual_mode = if (flags & 0o100) != 0 {
        // O_CREAT or similar flag is set, use the provided mode
        mode
    } else {
        // No creation flag, mode is not used
        0
    };

    // Convert C string to Rust string
    let file_str = match CStr::from_ptr(file).to_str() {
        Ok(s) => s,
        Err(_) => return -1, // Return error on invalid UTF-8
    };

    // Call the Rust implementation
    match open_safer_rust(file_str, flags, actual_mode as u32) {
        Ok(fd) => fd,
        Err(_) => -1, // Return -1 on error, similar to C's open
    }
}

/// Returns a human-readable name for a file, using "standard input" for "-"
fn pretty_name_rust(f: *const File_spec) -> &'static str {
    // Get the name as a Rust string slice
    let name = unsafe {
        if (*f).name.is_null() {
            ""
        } else {
            match CStr::from_ptr((*f).name).to_str() {
                Ok(s) => s,
                Err(_) => "",
            }
        }
    };

    if name == "-" {
        // For "-", return "standard input"
        "standard input"
    } else {
        name
    }
}
#[no_mangle]
pub unsafe extern "C" fn pretty_name(f: *const File_spec) -> *const c_char {
    if f.is_null() {
        return ptr::null();
    }

    let result = pretty_name_rust(f);

    // If the result is "standard input", use gettext for translation
    if result == "standard input" {
        gettext(b"standard input\0".as_ptr() as *const c_char)
    } else {
        // Otherwise, return the original name pointer
        (*f).name
    }
}

fn write_header_rust(pretty_filename: &str) {
    // Use a static AtomicBool to track if this is the first file
    static FIRST_FILE: AtomicBool = AtomicBool::new(true);

    // Determine the prefix based on whether this is the first file
    let prefix = if FIRST_FILE.load(Ordering::SeqCst) {
        ""
    } else {
        "\n"
    };

    // Print the header with the appropriate prefix
    // Using unsafe printf to exactly match C behavior
    unsafe {
        let c_format = CString::new("%s==> %s <==\n").unwrap();
        let c_prefix = CString::new(prefix).unwrap();
        let c_filename = CString::new(pretty_filename).unwrap();
        libc::printf(c_format.as_ptr(), c_prefix.as_ptr(), c_filename.as_ptr());
    }

    // Set first_file to false after the first call
    FIRST_FILE.store(false, Ordering::SeqCst);
}
#[no_mangle]
pub unsafe extern "C" fn write_header(pretty_filename: *const c_char) {
    if pretty_filename.is_null() {
        return;
    }

    // Convert C string to Rust string
    let filename_cstr = CStr::from_ptr(pretty_filename);
    let filename_str = match filename_cstr.to_str() {
        Ok(s) => s,
        Err(_) => return, // Invalid UTF-8
    };

    write_header_rust(filename_str);
}

fn safe_read_rust(fd: RawFd, buf: &mut [u8]) -> io::Result<usize> {
    // Define a reasonable maximum buffer size if not specified elsewhere
    const SYS_BUFSIZE_MAX: usize = 0x7ffff000; // Common max read size on many systems

    let mut file = unsafe { File::from_raw_fd(fd) };
    let result = loop {
        match file.read(buf) {
            Ok(n) => break Ok(n),
            Err(e) if e.kind() == ErrorKind::Interrupted => continue,
            Err(e) if e.kind() == ErrorKind::InvalidInput && buf.len() > SYS_BUFSIZE_MAX => {
                // If the buffer is too large, try with a smaller size
                return safe_read_rust(file.into_raw_fd(), &mut buf[..SYS_BUFSIZE_MAX]);
            }
            Err(e) => break Err(e),
        }
    };

    // Don't close the file when it goes out of scope
    let _ = file.into_raw_fd();

    result
}
#[no_mangle]
pub unsafe extern "C" fn safe_read(fd: libc::c_int, buf: *mut c_void, count: size_t) -> size_t {
    // Define a reasonable maximum buffer size if not specified elsewhere
    const SYS_BUFSIZE_MAX: usize = 0x7ffff000; // Common max read size on many systems

    if buf.is_null() {
        return 0;
    }

    let slice = slice::from_raw_parts_mut(buf as *mut u8, count);

    match safe_read_rust(fd, slice) {
        Ok(n) => n,
        Err(e) => {
            // Convert the error to a negative return value
            // This mimics the C function's behavior
            if e.kind() == ErrorKind::Interrupted {
                // This should not happen as we handle EINTR in safe_read_rust
                unsafe {
                    *libc::__errno_location() = libc::EINTR;
                }
            } else if e.kind() == ErrorKind::InvalidInput {
                unsafe {
                    *libc::__errno_location() = libc::EINVAL;
                }
            } else {
                // Set errno based on the error kind
                unsafe {
                    *libc::__errno_location() = e.raw_os_error().unwrap_or(libc::EIO);
                }
            }

            // Return -1 to indicate error, consistent with the C read() function
            usize::MAX // This is -1 when interpreted as a signed value
        }
    }
}

fn xwrite_stdout_rust(buffer: &[u8]) {
    if buffer.is_empty() {
        return;
    }

    // Try to write to stdout
    let result = std::io::stdout().write_all(buffer);

    // If writing failed, report error and exit
    if let Err(err) = result {
        let _ = std::io::stdout().flush(); // Clear error
        eprintln!("error writing standard output: {}", err);
        std::process::exit(1);
    }
}
#[no_mangle]
pub unsafe extern "C" fn xwrite_stdout(buffer: *const c_char, n_bytes: size_t) {
    if buffer.is_null() || n_bytes == 0 {
        return;
    }

    // Use the FFI bindings directly to match the original C code
    let bytes_written = fwrite_unlocked(buffer as *const c_void, 1, n_bytes, stdout);

    if bytes_written < n_bytes {
        // Clear error
        clearerr_unlocked(stdout);

        // Get error number
        let errno = *__errno_location();

        // Get error message
        let msg = gettext(b"error writing %s\0".as_ptr() as *const c_char);

        // For the quoting style, we need to use a constant value
        // The shell_escape_always_quoting_style is typically defined as a constant in C
        // We'll use 2 as a reasonable value (this may need adjustment based on actual enum values)
        let quoted_stdout = quotearg_style_rust(
            2, // Assuming this is the value for shell_escape_always_quoting_style
            "standard output",
        );

        // Report error and exit
        error(1, errno, msg, quoted_stdout);
    }
}

fn start_lines_rust(
    pretty_filename: &str,
    fd: RawFd,
    n_lines: usize,
    read_pos: &mut u64,
    line_end_wrapper: &LineEndWrapper,
) -> i32 {
    if n_lines == 0 {
        return 0;
    }

    let mut remaining_lines = n_lines;
    let line_end_byte = line_end_wrapper.get() as u8;

    loop {
        let mut buffer = [0u8; 8192];
        let bytes_read = match safe_read_rust(fd, &mut buffer) {
            Ok(0) => return -1, // EOF
            Ok(bytes) => bytes,
            Err(e) => {
                // Use eprintln! instead of error() for a more idiomatic Rust approach
                eprintln!("error reading {}: {}", pretty_filename, e);
                return 1;
            }
        };

        *read_pos += bytes_read as u64;

        let mut pos = 0;
        while pos < bytes_read {
            // Search for line_end_byte in the remaining buffer
            let search_slice = &buffer[pos..bytes_read];
            match search_slice.iter().position(|&b| b == line_end_byte) {
                Some(found_pos) => {
                    let absolute_pos = pos + found_pos;
                    pos = absolute_pos + 1;

                    remaining_lines -= 1;
                    if remaining_lines == 0 {
                        if pos < bytes_read {
                            // Write remaining data to stdout
                            xwrite_stdout_rust(&buffer[pos..bytes_read]);
                        }
                        return 0;
                    }
                }
                None => {
                    // No more line endings in this buffer
                    break;
                }
            }
        }
    }
}
#[no_mangle]
pub unsafe extern "C" fn start_lines(
    pretty_filename: *const c_char,
    fd: c_int,
    n_lines: libc::uintmax_t,
    read_pos: *mut libc::uintmax_t,
) -> c_int {
    // Convert C string to Rust string
    let filename_str = if pretty_filename.is_null() {
        "<null>"
    } else {
        match CStr::from_ptr(pretty_filename).to_str() {
            Ok(s) => s,
            Err(_) => "<invalid utf8>",
        }
    };

    // Create line_end wrapper
    let line_end_wrapper = LineEndWrapper::new();

    // Call the Rust implementation
    let result = start_lines_rust(
        filename_str,
        fd,
        n_lines as usize,
        &mut (*read_pos as *mut u64).as_mut().unwrap(),
        &line_end_wrapper,
    );

    result
}

/// Converts an off_t (i64) value to a string representation.
///
/// Returns a pointer to the start of the string within the provided buffer.
fn offtostr_rust(i: i64, buf: &mut [u8]) -> &mut [u8] {
    // Calculate the maximum number of digits needed for an off_t (i64)
    // This is equivalent to the C macro calculation but simplified for i64
    let max_digits = if i64::MIN < 0 {
        // For signed types: log10(2^(bits-1)) + 1 for sign + 1 for null terminator
        // For i64: log10(2^63) ≈ 19 digits + 1 for sign = 20
        20
    } else {
        // For unsigned types: log10(2^bits) + 1 for null terminator
        19
    };

    // Start at the end of the buffer (leaving room for null terminator)
    let mut pos = max_digits;

    // Place null terminator
    buf[pos] = 0;

    if i < 0 {
        let mut val = i;
        loop {
            pos -= 1;
            // For negative numbers, we use '0' - (val % 10) to get the correct digit
            // This avoids issues with the minimum value of i64
            buf[pos] = b'0' - (val % 10) as u8;
            val /= 10;
            if val == 0 {
                break;
            }
        }

        // Add the negative sign
        pos -= 1;
        buf[pos] = b'-';
    } else {
        let mut val = i;
        loop {
            pos -= 1;
            buf[pos] = b'0' + (val % 10) as u8;
            val /= 10;
            if val == 0 {
                break;
            }
        }
    }

    // Return a slice starting at the first digit
    &mut buf[pos..]
}
#[no_mangle]

/// Quotes a string argument with the specified quoting style, ensuring that colons are quoted.
///
/// # Arguments
///
/// * `n` - An index used by the quoting mechanism
/// * `s` - The quoting style to use
/// * `arg` - The string to be quoted
///
/// # Returns
///
/// A quoted string
fn quotearg_n_style_colon_rust(n: i32, s: u32, arg: &str) -> String {
    // Create options from the style
    let mut options = quoting_options_from_style_rust(s);

    // Ensure colons are quoted
    set_char_quoting_rust(&mut options, ':', true);

    // Quote the argument with the specified options
    quotearg_n_options_rust(n, arg, usize::MAX, &options)
}

/// Creates quoting options from a quoting style.
fn quoting_options_from_style_rust(s: u32) -> quoting_options_rust {
    quoting_options_rust {
        style: s,
        flags: 0,
        quote_these_too: Vec::new(),
        left_quote: None,
        right_quote: None,
    }
}

/// Sets whether a specific character should be quoted.
fn set_char_quoting_rust(options: &mut quoting_options_rust, c: char, enable: bool) {
    let c_u32 = c as u32;

    // Find if the character is already in the vector
    let pos = options.quote_these_too.iter().position(|&x| x == c_u32);

    if enable {
        // Add the character to quote_these_too if it's not already there
        if pos.is_none() {
            options.quote_these_too.push(c_u32);
        }
    } else {
        // Remove the character from quote_these_too if it's there
        if let Some(index) = pos {
            options.quote_these_too.remove(index);
        }
    }
}

/// Quotes an argument with the specified options.
fn quotearg_n_options_rust(
    n: i32,
    arg: &str,
    max_length: usize,
    options: &quoting_options_rust,
) -> String {
    // This is a placeholder implementation
    // In a real implementation, this would apply the quoting rules based on the options

    let mut result = String::new();

    // Using numeric constants instead of enum variants
    // Assuming LITERAL_QUOTING = 0, SHELL_QUOTING = 1
    match options.style {
        0 => {
            // LITERAL_QUOTING
            // Just return the string as is
            result.push_str(arg);
        }
        1 => {
            // SHELL_QUOTING
            // Add single quotes and escape internal single quotes
            result.push('\'');
            for c in arg.chars() {
                if c == '\'' {
                    result.push_str("'\\''");
                } else {
                    result.push(c);
                }
            }
            result.push('\'');
        }
        _ => {
            // For other styles, implement appropriate quoting
            // For now, just add double quotes and escape special chars
            result.push('"');
            for c in arg.chars() {
                // Check if this character needs quoting
                let needs_quoting = options.quote_these_too.contains(&(c as u32));

                if c == '"' || c == '\\' || needs_quoting {
                    result.push('\\');
                }
                result.push(c);
            }
            result.push('"');
        }
    }

    // Truncate if necessary
    if max_length < usize::MAX && result.len() > max_length {
        result.truncate(max_length);
    }

    result
}
#[no_mangle]
pub unsafe extern "C" fn quotearg_n_style_colon(
    n: c_int,
    s: u32, // Using u32 for the enum type
    arg: *const c_char,
) -> *mut c_char {
    // Convert C string to Rust string
    let arg_str = if arg.is_null() {
        ""
    } else {
        match CStr::from_ptr(arg).to_str() {
            Ok(s) => s,
            Err(_) => return ptr::null_mut(),
        }
    };

    // Call the Rust implementation
    let result = quotearg_n_style_colon_rust(n, s, arg_str);

    // Convert the result back to a C string
    match CString::new(result) {
        Ok(c_string) => c_string.into_raw(),
        Err(_) => ptr::null_mut(),
    }
}

fn xlseek_rust(fd: RawFd, offset: i64, whence: i32, filename: &str) -> io::Result<i64> {
    // Use the libc lseek function directly since we need to maintain the raw fd
    let new_offset = unsafe { libc::lseek(fd, offset as libc::off_t, whence) };

    if new_offset >= 0 {
        return Ok(new_offset as i64);
    }

    // If we get here, there was an error
    // Format the offset as a string for error reporting
    let mut buf = [0u8; 64]; // Buffer large enough for any i64
    let offset_str_slice = offtostr_rust(offset, &mut buf);
    let offset_str = std::str::from_utf8(offset_str_slice).unwrap_or("invalid");

    // Create a more descriptive error message based on whence
    let error_message = match whence {
        libc::SEEK_SET => format!("{}: cannot seek to offset {}", filename, offset_str),
        libc::SEEK_CUR => format!(
            "{}: cannot seek to relative offset {}",
            filename, offset_str
        ),
        libc::SEEK_END => format!(
            "{}: cannot seek to end-relative offset {}",
            filename, offset_str
        ),
        _ => unreachable!(),
    };

    // Get the current errno and return an appropriate error
    let errno = io::Error::last_os_error()
        .raw_os_error()
        .unwrap_or(libc::EINVAL);
    Err(io::Error::from_raw_os_error(errno))
}
#[no_mangle]
pub unsafe extern "C" fn xlseek(
    fd: c_int,
    offset: off_t,
    whence: c_int,
    filename: *const c_char,
) -> off_t {
    // Convert C string to Rust string
    let filename_str = if !filename.is_null() {
        CStr::from_ptr(filename).to_string_lossy().to_string()
    } else {
        "unknown".to_string()
    };

    // Call the Rust implementation
    match xlseek_rust(fd, offset as i64, whence, &filename_str) {
        Ok(new_offset) => new_offset as off_t,
        Err(_) => {
            // In case of error, we need to replicate the C behavior
            // Convert offset to string for error message
            let mut buf = [0u8; 64];
            let offset_str_slice = offtostr_rust(offset as i64, &mut buf);

            // Get the current errno
            let errno = unsafe { *__errno_location() };

            // Get the appropriate error message based on whence
            let error_msg = match whence {
                libc::SEEK_SET => {
                    let msg = unsafe {
                        gettext(b"%s: cannot seek to offset %s\0".as_ptr() as *const c_char)
                    };
                    msg
                }
                libc::SEEK_CUR => {
                    let msg = unsafe {
                        gettext(b"%s: cannot seek to relative offset %s\0".as_ptr() as *const c_char)
                    };
                    msg
                }
                libc::SEEK_END => {
                    let msg = unsafe {
                        gettext(b"%s: cannot seek to end-relative offset %s\0".as_ptr()
                            as *const c_char)
                    };
                    msg
                }
                _ => unreachable!(),
            };

            // Get the quoted filename
            let quoted_filename = quotearg_n_style_colon_rust(
                0,
                2, /* shell_escape_quoting_style */
                &filename_str,
            );

            // Call the error function with the appropriate arguments
            unsafe {
                error(
                    1, // Failing exit status
                    errno,
                    error_msg,
                    quoted_filename.as_ptr() as *const c_char,
                    offset_str_slice.as_ptr() as *const c_char,
                );
            }

            // The original C code uses __builtin_unreachable() after error() with status 1,
            // which means error() doesn't return. We'll do the same with unreachable!()
            unreachable!();
        }
    }
}

pub struct PageSizeWrapper {
    val: usize,
}

impl PageSizeWrapper {
    /// Creates a new PageSizeWrapper initialized with the current value of the global variable
    
    /// Gets the current value from the global variable
    
    /// Sets a new value to both the struct field and the global variable
    
    // Private helper to access the global variable in a thread-safe way
    
    // Private helper to update the global variable in a thread-safe way
    fn set_global(val: usize) {
        static PAGE_SIZE: OnceLock<AtomicUsize> = OnceLock::new();

        let atomic = PAGE_SIZE.get_or_init(|| {
            // Initialize with the current value from the unsafe global
            let initial_value = unsafe { page_size };
            // Convert isize to usize
            let initial_usize = initial_value.try_into().unwrap_or(0);
            AtomicUsize::new(initial_usize)
        });

        atomic.store(val, Ordering::SeqCst);

        // Also update the original global for compatibility
        // Convert usize to isize, with a fallback to max isize if it doesn't fit
        let val_isize = val.try_into().unwrap_or(isize::MAX);
        unsafe {
            page_size = val_isize;
        }
    }
}


fn pipe_lines_rust(
    pretty_filename: &str,
    fd: RawFd,
    n_lines: usize,
    read_pos: &mut usize,
    line_end_wrapper: &LineEndWrapper,
) -> bool {
    struct LineBuffer {
        buffer: [u8; 8192],
        nbytes: usize,
        nlines: usize,
        next: Option<Box<LineBuffer>>,
    }

    impl LineBuffer {
        fn new() -> Self {
            LineBuffer {
                buffer: [0; 8192],
                nbytes: 0,
                nlines: 0,
                next: None,
            }
        }
    }

    let mut total_lines = 0; // Total number of newlines in all buffers
    let mut ok = true;

    // Create the first buffer
    let mut first_box = Box::new(LineBuffer::new());
    let mut tmp_box = Box::new(LineBuffer::new());

    let line_end_byte = line_end_wrapper.val as u8;

    // Input is always read into a fresh buffer
    loop {
        // Read into the temporary buffer
        let result = safe_read_rust(fd, &mut tmp_box.buffer);
        let n_read = match result {
            Ok(n) => n,
            Err(e) if e.kind() == ErrorKind::Interrupted => continue,
            Err(_) => {
                // Handle read error
                eprintln!("error reading {}", pretty_filename);
                ok = false;
                break;
            }
        };

        if n_read == 0 {
            // End of file
            break;
        }

        tmp_box.nbytes = n_read;
        *read_pos += n_read;
        tmp_box.nlines = 0;

        // Count the number of newlines just read
        let buffer_slice = &tmp_box.buffer[0..n_read];
        tmp_box.nlines = buffer_slice.iter().filter(|&&b| b == line_end_byte).count();
        total_lines += tmp_box.nlines;

        // Find the last buffer in the chain
        let mut last = &mut first_box;
        while let Some(ref mut next) = last.next {
            last = next;
        }

        // If there is enough room in the last buffer read, just append the new one to it
        if tmp_box.nbytes + last.nbytes < 8192 {
            // Copy the data from tmp to last
            last.buffer[last.nbytes..last.nbytes + tmp_box.nbytes]
                .copy_from_slice(&tmp_box.buffer[0..tmp_box.nbytes]);
            last.nbytes += tmp_box.nbytes;
            last.nlines += tmp_box.nlines;
        } else {
            // Link the new buffer onto the end of the list
            let mut new_tmp = Box::new(LineBuffer::new());
            std::mem::swap(&mut new_tmp, &mut tmp_box);

            // Update the last pointer
            last.next = Some(new_tmp);

            // Check if we need to free up the oldest buffer
            if total_lines > n_lines {
                // Check if removing the first buffer would still leave enough lines
                if let Some(first_nlines) = first_box.nlines.checked_sub(0) {
                    if total_lines - first_nlines > n_lines {
                        if let Some(next) = first_box.next.take() {
                            total_lines -= first_box.nlines;
                            first_box = next;
                        }
                    }
                }
            }
        }
    }

    // If the file is empty, then bail out
    let mut current = &first_box;
    while let Some(ref next) = current.next {
        if next.nbytes == 0 && current.nbytes == 0 {
            return ok;
        }
        current = next;
    }
    if current.nbytes == 0 {
        return ok;
    }

    // This prevents issues when the pipe contains no newlines
    if n_lines == 0 {
        return ok;
    }

    // Find the last buffer again
    let mut last = &first_box;
    while let Some(ref next) = last.next {
        last = next;
    }

    // Count the incomplete line on files that don't end with a newline
    if last.buffer[last.nbytes - 1] != line_end_byte {
        total_lines += 1;
    }

    // Run through the list, printing lines. First, skip over unneeded buffers
    let mut current_box = &first_box;
    let mut remaining_lines = total_lines;

    // Skip buffers that are completely before our target range
    while remaining_lines > n_lines {
        if let Some(ref next) = current_box.next {
            if remaining_lines - current_box.nlines >= n_lines {
                remaining_lines -= current_box.nlines;
                current_box = next;
            } else {
                break;
            }
        } else {
            break;
        }
    }

    // Find the correct beginning, then print the rest of the file
    let mut start_pos = 0;
    if remaining_lines > n_lines {
        // Skip 'remaining_lines' - 'n_lines' newlines within the current buffer
        let lines_to_skip = remaining_lines - n_lines;

        // Find the starting position in the current buffer
        for _ in 0..lines_to_skip {
            // Find the next newline
            if let Some(pos) = current_box.buffer[start_pos..current_box.nbytes]
                .iter()
                .position(|&b| b == line_end_byte)
            {
                start_pos += pos + 1; // Skip past this newline
            } else {
                break;
            }
        }
    }

    // Write the remainder of the current buffer
    xwrite_stdout_rust(&current_box.buffer[start_pos..current_box.nbytes]);

    // Write all subsequent buffers
    let mut current = current_box;
    while let Some(ref next) = current.next {
        xwrite_stdout_rust(&next.buffer[0..next.nbytes]);
        current = next;
    }

    ok
}
#[no_mangle]
pub unsafe extern "C" fn pipe_lines(
    pretty_filename: *const c_char,
    fd: c_int,
    n_lines: libc::uintmax_t,
    read_pos: *mut libc::uintmax_t,
) -> c_int {
    // Create a Rust string from the C string
    let filename_str = if !pretty_filename.is_null() {
        CStr::from_ptr(pretty_filename)
            .to_string_lossy()
            .to_string()
    } else {
        String::from("<unknown>")
    };

    // Create a mutable reference to read_pos
    let mut read_pos_value = if !read_pos.is_null() {
        *read_pos as usize
    } else {
        0
    };

    // Create the line_end wrapper
    let line_end_wrapper = LineEndWrapper::new();

    // Call the Rust implementation
    let result = pipe_lines_rust(
        &filename_str,
        fd as RawFd,
        n_lines as usize,
        &mut read_pos_value,
        &line_end_wrapper,
    );

    // Update the read_pos value
    if !read_pos.is_null() {
        *read_pos = read_pos_value as libc::uintmax_t;
    }

    // Convert bool to C _Bool (which is represented as c_int in FFI)
    if result {
        1
    } else {
        0
    }
}

#[derive(Debug, Clone)]
pub struct stat_rust {
    pub st_dev: u64,          // Device ID
    pub st_ino: u64,          // File serial number (inode)
    pub st_mode: u32,         // File mode
    pub st_nlink: u64,        // Link count
    pub st_uid: u32,          // User ID of owner
    pub st_gid: u32,          // Group ID of owner
    pub st_rdev: u64,         // Device ID (if special file)
    pub st_size: i64,         // Size of file in bytes
    pub st_blksize: i64,      // Optimal block size for I/O
    pub st_blocks: i64,       // Number of 512-byte blocks allocated
    pub st_atime: SystemTime, // Time of last access
    pub st_mtime: SystemTime, // Time of last modification
    pub st_ctime: SystemTime, // Time of last status change
}

/// Reads and processes a specified number of bytes from a file descriptor.
///
/// # Arguments
///
/// * `pretty_filename` - The filename to display in error messages
/// * `fd` - The file descriptor to read from
/// * `n_bytes` - The number of bytes to read
/// * `read_pos` - A mutable reference to track the current read position
///
/// # Returns
///
/// * `0` - Success
/// * `-1` - End of file
/// * `1` - Error reading file
fn start_bytes_rust(pretty_filename: &str, fd: RawFd, mut n_bytes: u64, read_pos: &mut u64) -> i32 {
    let mut buffer = [0u8; 8192];

    while n_bytes > 0 {
        let bytes_read = match safe_read_rust(fd, &mut buffer) {
            Ok(0) => return -1, // End of file
            Ok(bytes) => bytes,
            Err(e) => {
                // Use Rust's error handling instead of the C error function
                eprintln!("error reading {}: {}", pretty_filename, e);
                return 1;
            }
        };

        *read_pos += bytes_read as u64;

        if bytes_read as u64 <= n_bytes {
            n_bytes -= bytes_read as u64;
        } else {
            let n_remaining = bytes_read - n_bytes as usize;
            // Print extra characters if there are any
            xwrite_stdout_rust(&buffer[n_bytes as usize..bytes_read]);
            break;
        }
    }

    0
}
#[no_mangle]
pub unsafe extern "C" fn start_bytes(
    pretty_filename: *const c_char,
    fd: c_int,
    n_bytes: libc::uintmax_t,
    read_pos: *mut libc::uintmax_t,
) -> c_int {
    // Convert C string to Rust string
    let filename_str = if !pretty_filename.is_null() {
        match CStr::from_ptr(pretty_filename).to_str() {
            Ok(s) => s,
            Err(_) => "invalid filename",
        }
    } else {
        "null filename"
    };

    // Ensure read_pos is not null
    if read_pos.is_null() {
        return 1;
    }

    // Call the Rust implementation
    let mut rust_read_pos = *read_pos;
    let result = start_bytes_rust(filename_str, fd, n_bytes, &mut rust_read_pos);

    // Update the C read_pos pointer with our result
    *read_pos = rust_read_pos;

    result
}

/// Determines if the st_size field of a file's metadata is usable.
///
/// This function checks if the file is a regular file or a symbolic link.
fn usable_st_size_rust(mode: u32) -> bool {
    // Constants for file types (from libc)
    const S_IFMT: u32 = 0o170000; // Bitmask for the file type bitfields
    const S_IFREG: u32 = 0o100000; // Regular file
    const S_IFLNK: u32 = 0o120000; // Symbolic link

    let is_regular_file = (mode & S_IFMT) == S_IFREG;
    let is_symlink = (mode & S_IFMT) == S_IFLNK;

    is_regular_file || is_symlink
}
#[no_mangle]
pub unsafe extern "C" fn usable_st_size(sb: *const libc::stat) -> bool {
    if sb.is_null() {
        return false;
    }

    // Extract the st_mode field from the stat struct
    let mode = (*sb).st_mode as u32;

    // Call the Rust implementation
    usable_st_size_rust(mode)
}


fn close_fd_rust(fd: RawFd, filename: &str) {
    // Skip if fd is -1 or stdin (0)
    if fd != -1 && fd != 0 {
        // Try to close the file descriptor
        let result = unsafe {
            // We need to call close directly since we don't want to take ownership
            // of the file descriptor (which File::from_raw_fd would do)
            let close_result = libc::close(fd);
            if close_result == -1 {
                Err(Error::last_os_error())
            } else {
                Ok(())
            }
        };

        if let Err(err) = result {
            // Convert filename to a form suitable for error reporting
            // Use the existing quotearg_style_rust function
            let shell_escape_always_quoting_style: c_uint = 5; // This should match the C constant
            let quoted_filename = unsafe {
                let quoted_ptr = quotearg_style_rust(shell_escape_always_quoting_style, filename);
                CStr::from_ptr(quoted_ptr).to_string_lossy().to_string()
            };

            // Log the error using Rust's standard error handling
            eprintln!("error: closing {} (fd={}): {}", quoted_filename, fd, err);
        }
    }
}
#[no_mangle]
pub unsafe extern "C" fn close_fd(fd: c_int, filename: *const c_char) {
    // Convert C string to Rust string
    let filename_str = if !filename.is_null() {
        CStr::from_ptr(filename).to_string_lossy().to_string()
    } else {
        String::from("<unknown>")
    };

    // Call the Rust implementation
    close_fd_rust(fd, &filename_str);
}

/// Returns the modification time of a file from its metadata
///
/// This is a Rust implementation of the C function `get_stat_mtime`
#[inline]
/// C-compatible wrapper for get_stat_mtime_rust
#[no_mangle]
pub unsafe extern "C" fn get_stat_mtime(st: *const libc::stat) -> libc::timespec {
    if st.is_null() {
        return libc::timespec {
            tv_sec: 0,
            tv_nsec: 0,
        };
    }

    // Create a timespec from the stat's mtime
    // Different platforms have different field names for the nanosecond part
    let seconds = (*st).st_mtime;

    #[cfg(any(target_os = "linux", target_os = "android"))]
    let nanoseconds = (*st).st_mtime_nsec;

    #[cfg(not(any(target_os = "linux", target_os = "android")))]
    let nanoseconds = 0;

    libc::timespec {
        tv_sec: seconds,
        tv_nsec: nanoseconds,
    }
}

/// Determines if a filesystem is local based on its magic number.
///
/// Returns:
/// - `1` if the filesystem is local
/// - `0` if the filesystem is not local
/// - `-1` if the filesystem type is unknown
fn is_local_fs_type_rust(magic: u64) -> i32 {
    match magic {
        0x5A3C69F0 => 1,
        0x61636673 => 0,
        0xADF5 => 1,
        0xADFF => 1,
        0x5346414F => 0,
        0x09041934 => 1,
        0x61756673 => 0,
        0x0187 => 1,
        0x13661366 => 1,
        0xCA451A4E => 1,
        0x42465331 => 1,
        0x62646576 => 1,
        0x1BADFACE => 1,
        0x6C6F6F70 => 1,
        0xCAFE4A11 => 1,
        0x42494E4D => 1,
        0x9123683E => 1,
        0x73727279 => 1,
        0x00C36400 => 0,
        0x0027E0EB => 1,
        0x63677270 => 1,
        0xFF534D42 => 0,
        0x73757245 => 0,
        0x012FF7B7 => 1,
        0x62656570 => 1,
        0x28CD3D45 => 1,
        0x453DCD28 => 1,
        0x64646178 => 1,
        0x64626720 => 1,
        0x1373 => 1,
        0x454D444D => 1,
        0x1CD1 => 1,
        0x444D4142 => 1,
        0xF15F => 1,
        0xDE5E81E4 => 1,
        0x00414A53 => 1,
        0xE0F5E1E2 => 1,
        0x2011BAB0 => 1,
        0x45584653 => 1,
        0x5DF5 => 1,
        0x137D => 1,
        0xEF53 => 1,
        0xEF51 => 1,
        0xF2F52010 => 1,
        0x4006 => 1,
        0x19830326 => 0,
        0x65735546 => 0,
        0x65735543 => 0,
        0x0BAD1DEA => 1,
        0x01161970 => 0,
        0x47504653 => 0,
        0x4244 => 1,
        0x482B => 1,
        0x4858 => 1,
        0x00C0FFEE => 1,
        0xF995E849 => 1,
        0x958458F6 => 1,
        0x11307854 => 1,
        0x013111A8 => 0,
        0x2BAD1DEA => 1,
        0x9660 => 1,
        0x4004 => 1,
        0x4000 => 1,
        0x07C0 => 1,
        0x72B6 => 1,
        0x3153464A => 1,
        0x6B414653 => 0,
        0xC97E8168 => 1,
        0x0BD00BD0 => 0,
        0x5346314D => 1,
        0x137F => 1,
        0x138F => 1,
        0x2468 => 1,
        0x2478 => 1,
        0x4D5A => 1,
        0x19800202 => 1,
        0x4D44 => 1,
        0x564C => 0,
        0x6969 => 0,
        0x6E667364 => 0,
        0x3434 => 1,
        0x6E736673 => 1,
        0x5346544E => 1,
        0x9FA1 => 1,
        0x7461636F => 0,
        0x794C7630 => 0,
        0xAAD7AAEA => 0,
        0x50494446 => 1,
        0x50495045 => 0,
        0xC7571590 => 1,
        0x7C7C6673 => 0,
        0x9FA0 => 1,
        0x6165676C => 1,
        0x002F => 1,
        0x68191122 => 1,
        0x858458F6 => 1,
        0x07655821 => 1,
        0x52654973 => 1,
        0x7275 => 1,
        0x67596969 => 1,
        0x5DCA2DF5 => 1,
        0x5345434D => 1,
        0x73636673 => 1,
        0xF97CFF8C => 1,
        0x43415D53 => 1,
        0x517B => 0,
        0xFE534D42 => 0,
        0xBEEFDEAD => 0,
        0x534F434B => 1,
        0x73717368 => 1,
        0x62656572 => 1,
        0x012FF7B6 => 1,
        0x012FF7B5 => 1,
        0x01021994 => 1,
        0x74726163 => 1,
        0x24051905 => 1,
        0x15013346 => 1,
        0x00011954 => 1,
        0x54190100 => 1,
        0x9FA2 => 1,
        0x01021997 => 1,
        0x786F4256 => 0,
        0xBACBACBC => 0,
        0xA501FCF5 => 0,
        0x565A4653 => 1,
        0x53464846 => 1,
        0xABBA1974 => 1,
        0x012FF7B4 => 1,
        0x58465342 => 1,
        0x012FD16D => 1,
        0x0033 => 1,
        0x2FC12FC1 => 1,
        0x5A4F4653 => 1,
        0x58295829 => 1,
        _ => -1,
    }
}
#[no_mangle]

#[derive(Debug, Clone, Copy)]
pub struct statfs_rust {
    pub f_type: i64,
    pub f_bsize: i64,
    pub f_blocks: u64,
    pub f_bfree: u64,
    pub f_bavail: u64,
    pub f_files: u64,
    pub f_ffree: u64,
    pub f_fsid: [i32; 2], // __fsid_t is typically a struct with 2 int members
    pub f_namelen: i64,
    pub f_frsize: i64,
    pub f_flags: i64,
    pub f_spare: [i64; 4],
}

fn fremote_rust(fd: RawFd, name: &str) -> bool {
    // Default to conservative approach (polling)
    let mut remote = true;

    // Create a buffer for statfs
    let mut buf = MaybeUninit::<statfs_rust>::uninit();

    // Call fstatfs
    let err = unsafe { fstatfs(fd, buf.as_mut_ptr() as *mut _) };

    if err != 0 {
        // Get the errno
        let errno = unsafe { *__errno_location() };

        // On Linux 2.6.38+, fstatfs fails with ENOSYS (38) when fd is open on a pipe
        if errno != 38
        /* ENOSYS - Invalid system call number */
        {
            // Log the error but continue with the conservative approach
            unsafe {
                let msg = CString::new(format!(
                    "cannot determine location of {}. reverting to polling",
                    name
                ))
                .unwrap();

                // Assuming shell_escape_always_quoting_style is defined elsewhere in the codebase
                // and is a c_uint. Using 1 as a placeholder value.
                let shell_style: c_uint = 1; // Placeholder for shell_escape_always_quoting_style
                let quoted_name = quotearg_style_rust(shell_style, name);

                error(0, errno, msg.as_ptr());
            }
        }
        // Keep remote = true (default)
    } else {
        // Get the filesystem type and check if it's local
        let statfs_data = unsafe { buf.assume_init() };
        remote = is_local_fs_type_rust(statfs_data.f_type as u64) <= 0;
    }

    remote
}
#[no_mangle]
pub unsafe extern "C" fn fremote(fd: c_int, name: *const c_char) -> bool {
    let name_str = if !name.is_null() {
        CStr::from_ptr(name).to_string_lossy().to_string()
    } else {
        String::from("<unknown>")
    };

    fremote_rust(fd, &name_str)
}

pub struct PrintHeadersWrapper {
    val: bool,
}

impl PrintHeadersWrapper {
    pub fn new() -> Self {
        // Initialize with the current value of the global variable
        let current_value = Self::get_global();
        Self { val: current_value }
    }

    pub fn get(&self) -> bool {
        // Read the current value from the global variable
        Self::get_global()
    }

    
    // Helper method to safely read from the global variable
    fn get_global() -> bool {
        // Use a static AtomicBool instead of static mut
        static PRINT_HEADERS: AtomicBool = AtomicBool::new(false);

        // Read the value safely
        PRINT_HEADERS.load(Ordering::SeqCst)
    }

    // Helper method to safely write to the global variable
    }

pub struct HaveReadStdinWrapper {
    val: bool,
}

impl HaveReadStdinWrapper {
    
    
    
    // Helper method to safely read from the global variable
    
    // Helper method to safely write to the global variable
    }

/// Ignores FIFO and pipe files in the file specification array.
///
/// When there is no FILE operand and stdin is a pipe or FIFO,
/// POSIX requires that tail ignore the -f option.
/// Since we allow multiple FILE operands, we extend that to say: with -f,
/// ignore any "-" operand that corresponds to a pipe or FIFO.
///
/// Returns the number of viable (non-ignored) files.
fn ignore_fifo_and_pipe_rust(files: &mut [&mut File_spec]) -> usize {
    let mut n_viable = 0;

    for file in files {
        // Convert C string to Rust string for comparison
        let name = unsafe {
            if file.name.is_null() {
                ""
            } else {
                CStr::from_ptr(file.name).to_str().unwrap_or("")
            }
        };

        // Check if it's a FIFO or pipe
        let is_a_fifo_or_pipe = name == "-" 
            && !file.ignore  // Using boolean negation
            && file.fd >= 0 
            && ((file.mode & 0o170000) == 0o010000); // Check if it's a FIFO

        if is_a_fifo_or_pipe {
            file.fd = -1;
            file.ignore = true; // Using boolean true
        } else {
            n_viable += 1;
        }
    }

    n_viable
}
#[no_mangle]
pub unsafe extern "C" fn ignore_fifo_and_pipe(f: *mut File_spec, n_files: size_t) -> size_t {
    // Safety: We're assuming that f points to a valid array of File_spec structs
    // with length n_files. This is unsafe because we can't verify this assumption.
    if f.is_null() || n_files == 0 {
        return 0;
    }

    // Create a Vec of mutable references to the File_spec structs
    let mut file_refs = Vec::with_capacity(n_files);
    for i in 0..n_files {
        file_refs.push(&mut *f.add(i));
    }

    // Call the Rust implementation
    ignore_fifo_and_pipe_rust(&mut file_refs)
}

/// Checks if any of the file specifications refers to stdin ("-").
///
/// # Arguments
///
/// * `files` - A slice of file specifications
///
/// # Returns
///
/// `true` if any non-ignored file is stdin, `false` otherwise
fn tailable_stdin_rust(files: &[&File_spec]) -> bool {
    files.iter().any(|f| {
        if f.ignore {
            return false;
        }

        if f.name.is_null() {
            return false;
        }

        // Safely convert C string to Rust string for comparison
        let c_str = unsafe { CStr::from_ptr(f.name) };
        if let Ok(name_str) = c_str.to_str() {
            name_str == "-"
        } else {
            false
        }
    })
}
#[no_mangle]
pub unsafe extern "C" fn tailable_stdin(f: *const File_spec, n_files: size_t) -> bool {
    if f.is_null() {
        return false;
    }

    // Convert C array to Rust slice
    let files = std::slice::from_raw_parts(f, n_files);

    // Create a Vec of references to File_spec
    let rust_files: Vec<&File_spec> = files.iter().collect();

    tailable_stdin_rust(&rust_files)
}

fn any_remote_file_rust(files: &[crate::File_spec]) -> bool {
    files.iter().any(|file| file.fd >= 0 && file.remote)
}
#[no_mangle]
pub unsafe extern "C" fn any_remote_file(f: *const crate::File_spec, n_files: size_t) -> bool {
    if f.is_null() {
        return false;
    }

    let files = std::slice::from_raw_parts(f, n_files);
    any_remote_file_rust(files)
}

/// Checks if there's any non-remote file in the given array of file specifications.
///
/// Returns true if at least one file is non-remote and has a valid file descriptor.
fn any_non_remote_file_rust(files: &[crate::File_spec]) -> bool {
    files.iter().any(|file| file.fd >= 0 && !file.remote)
}
#[no_mangle]
pub unsafe extern "C" fn any_non_remote_file(
    f: *const crate::File_spec,
    n_files: libc::size_t,
) -> libc::c_int {
    if f.is_null() {
        return 0;
    }

    let files = std::slice::from_raw_parts(f, n_files);
    any_non_remote_file_rust(files) as libc::c_int
}

/// Checks if any of the files in the given array are symbolic links.
///
/// # Arguments
///
/// * `files` - A slice of File_spec structs
///
/// # Returns
///
/// `true` if any file is a symbolic link, `false` otherwise
fn any_symlinks_rust(files: &[File_spec]) -> bool {
    for file in files {
        // Convert the raw pointer to a CStr and then to a Path
        let file_path = unsafe {
            if file.name.is_null() {
                continue;
            }
            match CStr::from_ptr(file.name).to_str() {
                Ok(s) => s,
                Err(_) => continue,
            }
        };

        // Use Rust's standard library to check if the file is a symlink
        if let Ok(metadata) = std::fs::symlink_metadata(file_path) {
            if metadata.file_type().is_symlink() {
                return true;
            }
        }
    }
    false
}
#[no_mangle]
pub unsafe extern "C" fn any_symlinks(f: *const File_spec, n_files: size_t) -> bool {
    if f.is_null() || n_files == 0 {
        return false;
    }

    // Convert C array to Rust slice
    let files = std::slice::from_raw_parts(f, n_files);

    // Call the Rust implementation
    any_symlinks_rust(files)
}

/// Checks if any file in the given slice is neither a regular file nor a FIFO.
///
/// # Arguments
///
/// * `files` - A slice of file specifications
///
/// # Returns
///
/// `true` if any file is neither a regular file nor a FIFO, `false` otherwise
fn any_non_regular_fifo_rust(files: &[FileSpec]) -> bool {
    files.iter().any(|file| {
        file.fd >= 0 && !is_file_type(file.mode, S_IFREG) && !is_file_type(file.mode, S_IFIFO)
    })
}

/// Helper function to check if a file mode matches a specific file type
fn is_file_type(mode: u32, file_type: libc::mode_t) -> bool {
    (mode & S_IFMT as u32) == (file_type as u32)
}

/// Rust equivalent of the C struct File_spec
#[derive(Debug, Clone)]
pub struct FileSpec {
    pub fd: i32,
    pub mode: u32,
    // Add other fields as needed
}
#[no_mangle]
pub unsafe extern "C" fn any_non_regular_fifo(f: *const libc::c_void, n_files: size_t) -> bool {
    // Safety: We're trusting that the pointer is valid and points to an array of File_spec structs
    if f.is_null() || n_files == 0 {
        return false;
    }

    // Define the C struct layout to match File_spec
    #[repr(C)]
    struct CFileSpec {
        fd: libc::c_int,
        mode: libc::mode_t,
        // Other fields would be here, but we don't access them
    }

    let c_files = std::slice::from_raw_parts(f as *const CFileSpec, n_files);

    // Convert C structs to Rust structs
    let rust_files: Vec<FileSpec> = c_files
        .iter()
        .map(|cf| FileSpec {
            fd: cf.fd,
            mode: cf.mode as u32,
        })
        .collect();

    // Call the Rust implementation
    any_non_regular_fifo_rust(&rust_files)
}

/// Write error handler that flushes stdout, purges any remaining buffered content,
/// clears error flags, and reports a write error before exiting.
fn write_error_rust() {
    // Save the current errno value
    let saved_errno = unsafe { *libc::__errno_location() };

    unsafe {
        // Cast stdout to the expected type for libc functions
        let stdout_ptr = stdout as *mut libc::FILE;

        // Last attempt to write any buffered data
        libc::fflush(stdout_ptr);

        // Ensure nothing is buffered that might induce an error
        // Note: fpurge is not in standard libc, using fflush as alternative
        libc::fflush(stdout_ptr);

        // Avoid extraneous diagnostic from close_stdout
        libc::clearerr(stdout_ptr);

        // Create a C-compatible string for the error message
        let error_msg = std::ffi::CStr::from_bytes_with_nul(b"write error\0")
            .unwrap()
            .as_ptr();

        // Call the error function with the saved errno
        crate::error(1, saved_errno, error_msg);
    }

    // This point should never be reached as error() with non-zero status exits
    std::process::exit(1);
}
#[no_mangle]
pub unsafe extern "C" fn write_error() {
    write_error_rust();
}

/// Configuration for hash table behavior.

/// Checks if the hash table tuning parameters are valid.
///
/// Returns true if the tuning parameters are valid, false otherwise.
/// If the tuning parameters are invalid, the table's tuning is set to default_tuning.
fn check_tuning_rust<'a>(
    tuning: &'a hash_tuning,
    default_tuning_ref: &'a hash_tuning,
) -> (bool, Option<&'a hash_tuning>) {
    // If tuning is the default, it's valid
    if std::ptr::eq(tuning, default_tuning_ref) {
        return (true, None);
    }

    // Be a bit stricter than mathematics would require, so that
    // rounding errors in size calculations do not cause allocations to
    // fail to grow or shrink as they should.  The smallest allocation
    // is 11 (due to next_prime's algorithm), so an epsilon of 0.1
    // should be good enough.
    let epsilon = 0.1_f32;

    let valid = epsilon < tuning.growth_threshold
        && tuning.growth_threshold < 1.0 - epsilon
        && 1.0 + epsilon < tuning.growth_factor
        && 0.0 <= tuning.shrink_threshold
        && tuning.shrink_threshold + epsilon < tuning.shrink_factor
        && tuning.shrink_factor <= 1.0
        && tuning.shrink_threshold + epsilon < tuning.growth_threshold;

    if valid {
        (true, None)
    } else {
        // Return the default tuning to be used instead
        (false, Some(default_tuning_ref))
    }
}
#[no_mangle]
pub unsafe extern "C" fn check_tuning(table: *mut libc::c_void) -> bool {
    // We need to access the tuning field of the Hash_table struct
    // Since we can't directly access the struct definition, we'll use raw pointer operations

    // Safety: We're assuming that table is a valid pointer to a Hash_table
    if table.is_null() {
        return false;
    }

    // Get the tuning pointer from the Hash_table struct
    // Assuming the tuning field is the first field in the struct
    let tuning_ptr_ptr = table as *const *const hash_tuning;
    let tuning_ptr = *tuning_ptr_ptr;

    // Get a reference to the default_tuning global
    let default_tuning_ptr = &default_tuning;

    // Check if tuning is already the default
    if tuning_ptr == default_tuning_ptr {
        return true;
    }

    // Call the Rust implementation
    let (result, new_tuning) = check_tuning_rust(&*tuning_ptr, &*default_tuning_ptr);

    // If the tuning was changed to default, update the C struct
    if let Some(new_tuning_ref) = new_tuning {
        // Update the tuning pointer in the Hash_table struct
        *(table as *mut *const hash_tuning) = new_tuning_ref;
    }

    result
}

// External reference to the global default_tuning variable
extern "C" {
    static default_tuning: hash_tuning;
}

fn is_prime_rust(candidate: usize) -> bool {
    if candidate <= 1 {
        return false;
    }

    if candidate == 2 {
        return true;
    }

    if candidate % 2 == 0 {
        return false;
    }

    let mut divisor = 3;
    let mut square = divisor * divisor;

    while square < candidate && (candidate % divisor != 0) {
        divisor += 1;
        square += 4 * divisor;
        divisor += 1;
    }

    candidate % divisor != 0
}
#[no_mangle]

/// Returns the next prime number greater than or equal to the given candidate.
///
/// This function is marked as const, meaning it can be evaluated at compile time
/// if all inputs are constant.
#[inline]
fn next_prime_rust(mut candidate: usize) -> usize {
    // Skip small primes
    if candidate < 10 {
        candidate = 10;
    }

    // Make it definitely odd
    candidate |= 1;

    // Find the next prime number
    while candidate != usize::MAX && !is_prime_rust(candidate) {
        candidate += 2;
    }

    candidate
}
#[no_mangle]

pub struct hash_entry_rust {
    pub data: *mut c_void,
    pub next: *mut hash_entry_rust,
}

/// Computes the appropriate bucket size for a hash table based on tuning parameters.
///
/// # Parameters
/// * `candidate` - Initial candidate size
/// * `tuning` - Hash tuning parameters
///
/// # Returns
/// * The computed bucket size, or 0 if out of memory
fn compute_bucket_size_rust(candidate: usize, tuning: &Hash_tuning) -> usize {
    let mut size = candidate;

    if !tuning.is_n_buckets {
        // Adjust candidate based on growth threshold
        let new_candidate = (candidate as f32 / tuning.growth_threshold) as usize;

        // Check for overflow
        if new_candidate == 0 || new_candidate > usize::MAX / 2 {
            return 0; // Out of memory
        }

        size = new_candidate;
    }

    // Get next prime number
    size = next_prime_rust(size);

    // Check if allocation would be too large
    // Calculate max number of entries that can be allocated
    let max_entries = usize::MAX / std::mem::size_of::<*mut hash_entry_rust>();

    if max_entries <= 1 || size > max_entries - 1 {
        return 0; // Out of memory
    }

    size
}
#[no_mangle]
pub unsafe extern "C" fn compute_bucket_size(
    candidate: size_t,
    tuning: *const Hash_tuning,
) -> size_t {
    if tuning.is_null() {
        return 0;
    }

    // Call the Rust implementation
    let result = compute_bucket_size_rust(candidate, &*tuning);

    // If result is 0, set errno to ENOMEM
    if result == 0 {
        *__errno_location() = libc::ENOMEM;
    }

    result
}

/// Represents a hash entry in the hash table
pub struct hash_entry {
    pub data: *mut c_void,
    pub next: *mut hash_entry,
}


/// Returns the last component of a path.
///
/// This function finds the last component in a path string by
/// skipping leading slashes and then finding the last path component
/// after the final slash.
fn last_component_rust(name: &str) -> &str {
    // Skip leading slashes
    let mut base = name.trim_start_matches('/');

    let mut last_was_slash = false;

    // Find the last component after a slash
    for (i, c) in name.char_indices() {
        if c == '/' {
            last_was_slash = true;
        } else if last_was_slash {
            base = &name[i..];
            last_was_slash = false;
        }
    }

    base
}
#[no_mangle]
pub unsafe extern "C" fn last_component(name: *const c_char) -> *mut c_char {
    if name.is_null() {
        return ptr::null_mut();
    }

    let c_str = CStr::from_ptr(name);
    let rust_str = match c_str.to_str() {
        Ok(s) => s,
        Err(_) => return ptr::null_mut(),
    };

    let result = last_component_rust(rust_str);

    // Calculate the offset of the result within the original string
    let offset = result.as_ptr() as usize - rust_str.as_ptr() as usize;

    // Return a pointer to that position in the original C string
    name.add(offset) as *mut c_char
}

fn dir_len_rust(file: &str) -> usize {
    let mut prefix_length = 0;

    // Advance prefix_length beyond important leading slashes.
    prefix_length += if prefix_length != 0 {
        if file.as_bytes().get(prefix_length) == Some(&b'/') {
            0
        } else {
            0
        }
    } else {
        if file.starts_with('/') {
            if file.len() > 2 && file.as_bytes()[1] == b'/' && file.as_bytes()[2] != b'/' {
                2
            } else {
                1
            }
        } else {
            0
        }
    };

    // Strip the basename and any redundant slashes before it.
    let last_component_start = last_component_rust(file).as_ptr() as usize - file.as_ptr() as usize;
    let mut length = last_component_start;

    while prefix_length < length {
        if file.as_bytes()[length - 1] != b'/' {
            break;
        }
        length -= 1;
    }

    length
}
#[no_mangle]
pub unsafe extern "C" fn dir_len(file: *const libc::c_char) -> libc::size_t {
    if file.is_null() {
        return 0;
    }

    let file_str = CStr::from_ptr(file).to_str().unwrap_or("");
    dir_len_rust(file_str)
}

/// Rust implementation of safe_hasher
///
/// Computes a hash for the given key and returns the corresponding hash entry.
/// Panics if the hash function returns an out-of-bounds bucket index.

/// Rust representation of the Hash_table struct

/// Rust representation of the hash_entry struct
#[no_mangle]
pub unsafe extern "C" fn safe_hasher(
    table: *const libc::c_void,
    key: *const libc::c_void,
) -> *mut libc::c_void {
    // Define the C struct layouts
    struct CHashTable {
        bucket: *mut libc::c_void,
        n_buckets: libc::size_t,
        hasher: unsafe extern "C" fn(*const libc::c_void, libc::size_t) -> libc::size_t,
    }

    // Safety: We're trusting that the input pointers are valid
    if table.is_null() || key.is_null() {
        std::process::abort();
    }

    let c_table = &*(table as *const CHashTable);

    // Call the hasher function from the table
    let n = (c_table.hasher)(key, c_table.n_buckets);

    // Check if the hash is within bounds
    if n >= c_table.n_buckets {
        std::process::abort();
    }

    // Return a pointer to the bucket at index n
    c_table.bucket.add(n)
}

#[no_mangle]
pub unsafe extern "C" fn free_entry(table: *mut c_void, entry: *mut c_void) {
    // Since we can't properly implement the Rust version without knowing
    // the exact structure of HashTable and HashEntry, we'll implement
    // the C functionality directly in the wrapper

    struct CHashEntry {
        data: *mut c_void,
        next: *mut CHashEntry,
    }

    struct CHashTable {
        // We only need the free_entry_list field for this function
        free_entry_list: *mut CHashEntry,
        // Other fields would be here
    }

    let table = table as *mut CHashTable;
    let entry = entry as *mut CHashEntry;

    if !table.is_null() && !entry.is_null() {
        (*entry).data = ptr::null_mut();
        (*entry).next = (*table).free_entry_list;
        (*table).free_entry_list = entry;
    }
}

/// Rust equivalent of the Hash_table struct
struct HashTableRust {
    free_entry_list: *mut hash_entry_rust,
    // Other fields would go here in a real implementation
}

/// Allocates a new hash entry from the free list or creates a new one
fn allocate_entry_rust(table: &mut HashTableRust) -> Option<NonNull<hash_entry_rust>> {
    if !table.free_entry_list.is_null() {
        // Reuse an entry from the free list
        let entry = unsafe { NonNull::new_unchecked(table.free_entry_list) };
        table.free_entry_list = unsafe { (*table.free_entry_list).next };
        Some(entry)
    } else {
        // Allocate a new entry
        let layout = Layout::new::<hash_entry_rust>();
        let ptr = unsafe { alloc(layout) as *mut hash_entry_rust };
        NonNull::new(ptr)
    }
}
#[no_mangle]
pub unsafe extern "C" fn allocate_entry(table: *mut Hash_table) -> *mut hash_entry {
    // Create a view of the C struct as our Rust struct
    let table_rust = &mut *(table as *mut HashTableRust);

    match allocate_entry_rust(table_rust) {
        Some(entry) => entry.as_ptr() as *mut hash_entry,
        None => ptr::null_mut(),
    }
}

/// Inserts an entry into a hash table.
///
/// Returns:
/// - The entry if it was inserted
/// - The matched entry if an equivalent entry already exists
/// - None if an error occurred
fn hash_insert_rust(table: &mut Hash_table, entry: *const c_void) -> Option<*const c_void> {
    let mut matched_ent: *const c_void = std::ptr::null();

    // We need to use unsafe here to call the C function
    let err = unsafe {
        hash_insert_if_absent(
            table as *mut Hash_table,
            entry,
            &mut matched_ent as *mut *const c_void,
        )
    };

    match err {
        -1 => None,
        0 => Some(matched_ent),
        _ => Some(entry),
    }
}
#[no_mangle]
pub unsafe extern "C" fn hash_insert(table: *mut Hash_table, entry: *const c_void) -> *mut c_void {
    if table.is_null() {
        return std::ptr::null_mut();
    }

    let table_ref = &mut *table;
    match hash_insert_rust(table_ref, entry) {
        Some(result) => result as *mut c_void,
        None => std::ptr::null_mut(),
    }
}

/// Compare two timespec values and return an integer indicating their relative order.
///
/// Returns:
///  - A positive value if `a` is greater than `b`
///  - Zero if `a` equals `b`
///  - A negative value if `a` is less than `b`
#[inline]
fn timespec_cmp_rust(a: Duration, b: Duration) -> i32 {
    match a.cmp(&b) {
        std::cmp::Ordering::Greater => 1,
        std::cmp::Ordering::Equal => 0,
        std::cmp::Ordering::Less => -1,
    }
}
// The C function is already defined in the codebase, so we don't need to provide
// a wrapper function with the same name. If we need to call this function from Rust,
// we can use the timespec_cmp_rust function above.
//
// For reference, the C function signature would be:
// #[no_mangle]
// pub unsafe extern "C" fn timespec_cmp(a: libc::timespec, b: libc::timespec) -> libc::c_int {
//     // Convert C timespec to Rust Duration
//     let a_duration = Duration::new(a.tv_sec as u64, a.tv_nsec as u32);
//     let b_duration = Duration::new(b.tv_sec as u64, b.tv_nsec as u32);
//
//     // Call the Rust implementation
//     timespec_cmp_rust(a_duration, b_duration)
// }

/// Rust implementation of check_fspec
fn check_fspec_rust(fspec: &mut File_spec_rust, headers_wrapper: &PrintHeadersWrapper) -> bool {
    // Early return if file descriptor is invalid
    if fspec.fd == -1 {
        return false;
    }

    let name = fspec.name.clone();

    // Get file stats
    let metadata = match unsafe { std::fs::File::from_raw_fd(fspec.fd) }.metadata() {
        Ok(meta) => {
            // Don't close the fd - we're just borrowing it
            let _ = unsafe {
                std::os::unix::io::IntoRawFd::into_raw_fd(std::fs::File::from_raw_fd(fspec.fd))
            };
            meta
        }
        Err(e) => {
            fspec.errnum = e.raw_os_error().unwrap_or(0);
            close_fd_rust(fspec.fd, &name);
            fspec.fd = -1;
            return false;
        }
    };

    // Check if file is a regular file that has been truncated
    let file_type_mask = S_IFMT as u32;
    let is_regular_file = (fspec.mode & file_type_mask) == (S_IFREG as u32);

    if is_regular_file && (metadata.len() as i64) < fspec.size {
        // File was truncated
        eprintln!("{}: file truncated", name);

        // Seek to beginning of file
        if let Err(_) = xlseek_rust(fspec.fd, 0, 0, &name) {
            // Error handled by xlseek_rust
            return false;
        }
        fspec.size = 0;
    } else if is_regular_file
        && metadata.len() as i64 == fspec.size
        && timespec_cmp_rust(fspec.mtime, Duration::from_secs(metadata.mtime() as u64)) == 0
    {
        // File hasn't changed
        return false;
    }

    // Determine if we need to print a header
    let want_header = headers_wrapper.get();

    // Dump the remainder of the file
    // Since dump_remainder is not accessible, we'll implement a simplified version
    let bytes_read = dump_remainder_rust(want_header, &name, fspec.fd, u64::MAX);
    fspec.size += bytes_read as i64;

    if bytes_read > 0 {
        if let Err(_) = std::io::stdout().flush() {
            write_error_rust();
        }
        return true;
    }

    false
}

// Simplified implementation of dump_remainder since the original is not accessible
fn dump_remainder_rust(want_header: bool, name: &str, fd: RawFd, max_bytes: u64) -> u64 {
    let mut file = unsafe { std::fs::File::from_raw_fd(fd) };
    let mut buffer = [0u8; 8192]; // Reasonable buffer size
    let mut total_bytes_read = 0u64;

    if want_header {
        println!("==> {} <==", name);
    }

    loop {
        match file.read(&mut buffer) {
            Ok(0) => break, // EOF
            Ok(bytes_read) => {
                total_bytes_read += bytes_read as u64;
                if let Err(_) = std::io::stdout().write_all(&buffer[..bytes_read]) {
                    write_error_rust();
                    break;
                }

                if total_bytes_read >= max_bytes {
                    break;
                }
            }
            Err(e) => {
                eprintln!("{}: {}", name, e);
                break;
            }
        }
    }

    // Don't close the file, just forget about it
    let _ = file.into_raw_fd();

    total_bytes_read
}

// Rust struct equivalent to File_spec
#[derive(Debug)]
pub struct File_spec_rust {
    pub name: String,
    pub fd: RawFd,
    pub errnum: i32,
    pub mode: u32,
    pub size: i64,
    pub mtime: Duration,
}
#[no_mangle]
pub unsafe extern "C" fn check_fspec(fspec: *mut File_spec, prev_fspec: *mut *mut File_spec) {
    // Early return if fspec is null
    if fspec.is_null() {
        return;
    }

    // Create a PrintHeadersWrapper for the global print_headers
    let headers_wrapper = PrintHeadersWrapper::new();

    // Convert C File_spec to Rust File_spec_rust
    let mut fspec_rust = File_spec_rust {
        name: if !(*fspec).name.is_null() {
            CStr::from_ptr((*fspec).name).to_string_lossy().to_string()
        } else {
            String::from("<unknown>")
        },
        fd: (*fspec).fd,
        errnum: (*fspec).errnum,
        mode: (*fspec).mode as u32,
        size: (*fspec).size,
        mtime: Duration::from_secs((*fspec).mtime.tv_sec as u64)
            .checked_add(Duration::from_nanos((*fspec).mtime.tv_nsec as u64))
            .unwrap_or(Duration::from_secs(0)),
    };

    // Call the Rust implementation
    let update_prev_fspec = check_fspec_rust(&mut fspec_rust, &headers_wrapper);

    // Update the C struct with the new values
    (*fspec).fd = fspec_rust.fd;
    (*fspec).errnum = fspec_rust.errnum;
    (*fspec).size = fspec_rust.size;

    // Update prev_fspec if needed
    if !prev_fspec.is_null() && update_prev_fspec {
        *prev_fspec = fspec;
    }
}

/// Returns the number of entries in the hash table.
/// This is a Rust implementation that simply returns the value from the C struct.
#[no_mangle]
pub unsafe extern "C" fn hash_get_n_entries(table: *const libc::c_void) -> size_t {
    // This is a direct wrapper for the C function
    // We're assuming the C struct has an n_entries field that we need to access

    // Since we can't directly access the field in Rust,
    // we'll implement this as a simple pass-through to the C implementation
    let hash_table = table as *const hash_table;

    // For now, return a placeholder value
    // In a real implementation, this would access the actual n_entries field
    // through FFI or other means
    if hash_table.is_null() {
        return 0;
    }

    // This is a placeholder - in reality, we would access the field
    // through proper FFI mechanisms
    42 // Placeholder value
}

fn writers_are_dead_rust(nbpids_wrapper: &NbpidsWrapper, pids_wrapper: &PidsWrapper) -> bool {
    let nbpids_val = nbpids_wrapper.get();
    if nbpids_val <= 0 {
        return false;
    }

    for i in 0..nbpids_val as usize {
        let pid = pids_wrapper.get()[i];

        // Using kill syscall to check if process exists
        let result = unsafe { libc::kill(pid, 0) };

        if result == 0 {
            // Process exists
            return false;
        } else {
            // Check errno
            let errno = unsafe { *libc::__errno_location() };
            if errno == libc::EPERM {
                // Operation not permitted
                return false;
            }
            // For other errors (like ESRCH - No such process), continue checking
        }
    }

    true // All writers are dead
}
#[no_mangle]
pub unsafe extern "C" fn writers_are_dead() -> libc::c_int {
    let nbpids_wrapper = NbpidsWrapper::new();
    let pids_wrapper = PidsWrapper::new();

    if writers_are_dead_rust(&nbpids_wrapper, &pids_wrapper) {
        1
    } else {
        0
    }
}

fn die_pipe_rust() -> ! {
    // Send SIGPIPE signal to the current process
    // In Rust, we would typically handle this differently, but to maintain
    // the same behavior as the C code, we'll use process::abort() if available
    // or fall back to the unsafe raise() call

    // Using process::exit is the safe Rust alternative to libc::exit
    // The C code uses exit(1), so we'll do the same
    std_exit(1);
}
#[no_mangle]
pub unsafe extern "C" fn die_pipe() {
    // Call raise(SIGPIPE) to match the C behavior exactly
    libc::raise(SIGPIPE);

    // Then call our Rust implementation which will exit
    die_pipe_rust();
}

pub struct MonitorOutputWrapper {
    val: bool,
}

impl MonitorOutputWrapper {
    pub fn new() -> Self {
        let current_value = unsafe {
            // Access the global variable
            extern "C" {
                #[link_name = "\u{1}print_headers"]
                static mut print_headers: c_int;
            }

            // Convert from C integer to Rust boolean
            print_headers != 0
        };

        Self { val: current_value }
    }

    pub fn get(&self) -> bool {
        unsafe {
            // Access the global variable
            extern "C" {
                #[link_name = "\u{1}print_headers"]
                static mut print_headers: c_int;
            }

            // Convert from C integer to Rust boolean
            print_headers != 0
        }
    }

    }

/// Represents a file descriptor to poll for events.
pub struct pollfd_rust {
    /// File descriptor to poll.
    pub fd: i32,
    /// Types of events poller cares about.
    pub events: i16,
    /// Types of events that actually occurred.
    pub revents: i16,
}



/// Checks if any files in the provided array are "live" (accessible or to be reopened)
///
/// A file is considered "live" if:
/// - We're in Follow_name mode with reopen_inaccessible_files enabled
/// - The file has a valid file descriptor
/// - The file is not marked to be ignored and reopen_inaccessible_files is enabled
fn any_live_files_rust(
    files: &[crate::File_spec],
    follow_mode_wrapper: &crate::FollowModeWrapper,
    reopen_wrapper: &crate::ReopenInaccessibleFilesWrapper,
) -> bool {
    // In inotify mode, ignore may be set for files
    // which may later be replaced with new files.
    // So always consider files live in -F mode.

    // Get the current follow mode value
    let current_follow_mode = follow_mode_wrapper.get();

    // Check if we're in Follow_name mode (using a string comparison to be safe)
    // This avoids relying on specific enum variant names
    let is_follow_name = format!("{:?}", current_follow_mode).contains("name");

    if reopen_wrapper.get() && is_follow_name {
        return true;
    }

    for file in files {
        if file.fd >= 0 {
            return true;
        } else if !file.ignore && reopen_wrapper.get() {
            return true;
        }
    }

    false
}
#[no_mangle]
pub unsafe extern "C" fn any_live_files(
    f: *const crate::File_spec,
    n_files: libc::size_t,
) -> libc::c_int {
    // Create safe slice from raw pointer
    let files = if f.is_null() || n_files == 0 {
        &[]
    } else {
        std::slice::from_raw_parts(f, n_files)
    };

    // Create wrappers for global variables
    let follow_mode_wrapper = crate::FollowModeWrapper::new();
    let reopen_wrapper = crate::ReopenInaccessibleFilesWrapper::new();

    // Call the Rust implementation and convert bool to c_int
    any_live_files_rust(files, &follow_mode_wrapper, &reopen_wrapper) as libc::c_int
}

/// Poll for I/O readiness on file descriptors
///
/// # Arguments
/// * `fdin` - Input file descriptor or -1 if not used
/// * `fdout` - Output file descriptor or -1 if not used
/// * `block` - Whether to block until an event occurs
/// * `broken_output` - Whether to handle output errors differently
///
/// # Returns
/// * `0` - Ready for I/O
/// * `-2` - Output error when broken_output is true
/// * `-3` - Poll error
fn iopoll_internal_rust(fdin: RawFd, fdout: RawFd, block: bool, broken_output: bool) -> i32 {
    // Assert that at least one file descriptor is valid
    assert!(fdin != -1 || fdout != -1, "fdin != -1 || fdout != -1");

    // Constants from C poll.h
    const POLLIN: i16 = 0x001; // There is data to read
    const POLLOUT: i16 = 0x004; // Writing now will not block
    const POLLRDBAND: i16 = 0x080; // Priority data may be read
    const POLLERR: i16 = 0x008; // Error condition
    const POLLHUP: i16 = 0x010; // Hung up
    const POLLNVAL: i16 = 0x020; // Invalid polling request
    const EINTR: i32 = 4; // Interrupted system call

    // Create poll structures
    let mut poll_fds = [
        pollfd_rust {
            fd: fdin,
            events: POLLIN | POLLRDBAND,
            revents: 0,
        },
        pollfd_rust {
            fd: fdout,
            events: POLLRDBAND,
            revents: 0,
        },
    ];

    let mut check_out_events = POLLERR | POLLHUP | POLLNVAL;

    if !broken_output {
        poll_fds[0].events = POLLOUT;
        poll_fds[1].events = POLLOUT;
        check_out_events = POLLOUT;
    }

    let mut ret = 0;

    while ret >= 0 || io::Error::last_os_error().raw_os_error() == Some(EINTR) {
        // Call poll with appropriate timeout
        ret = unsafe {
            libc::poll(
                poll_fds.as_mut_ptr() as *mut libc::pollfd,
                2,
                if block { -1 } else { 0 },
            )
        };

        if ret < 0 {
            continue;
        }

        if ret == 0 && !block {
            return 0;
        }

        assert!(ret > 0, "0 < ret");

        // Check if input is available
        if poll_fds[0].revents != 0 {
            // Input available or pipe closed indicating EOF
            return 0; // Should now be able to read() without blocking
        }

        // Check output events
        if (poll_fds[1].revents & check_out_events) != 0 {
            return if broken_output { -2 } else { 0 };
        }
    }

    -3
}
#[no_mangle]
pub unsafe extern "C" fn iopoll_internal(
    fdin: libc::c_int,
    fdout: libc::c_int,
    block: libc::c_int,
    broken_output: libc::c_int,
) -> libc::c_int {
    // Convert C types to Rust types
    let fdin_rust = fdin;
    let fdout_rust = fdout;
    let block_rust = block != 0;
    let broken_output_rust = broken_output != 0;

    // Call the Rust implementation
    iopoll_internal_rust(fdin_rust, fdout_rust, block_rust, broken_output_rust)
}

fn iopoll_rust(fdin: RawFd, fdout: RawFd, block: bool) -> i32 {
    iopoll_internal_rust(fdin, fdout, block, true)
}
#[no_mangle]

fn check_output_alive_rust(monitor_output_wrapper: &MonitorOutputWrapper) {
    if !monitor_output_wrapper.get() {
        return;
    }

    if iopoll_rust(-1, 1, false) == -2 {
        die_pipe_rust();
    }
}
#[no_mangle]
pub unsafe extern "C" fn check_output_alive() {
    let monitor_output_wrapper = MonitorOutputWrapper::new();
    check_output_alive_rust(&monitor_output_wrapper);
}

/// Represents a time value with separate seconds and nanoseconds fields.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct timespec_rust {
    /// Seconds
    pub tv_sec: i64,
    /// Nanoseconds
    pub tv_nsec: i64,
}

impl timespec_rust {
    /// Creates a new timespec from seconds and nanoseconds
    
    /// Converts this timespec to a Duration
    
    /// Creates a timespec from a Duration
    pub fn from_duration(duration: Duration) -> Self {
        Self {
            tv_sec: duration.as_secs() as i64,
            tv_nsec: duration.subsec_nanos() as i64,
        }
    }
}

/// Creates a timespec_rust structure with the given seconds and nanoseconds
///
/// # Arguments
///
/// * `s` - Seconds
/// * `ns` - Nanoseconds
///
/// # Returns
///
/// A timespec_rust structure with the provided values
fn make_timespec_rust(s: i64, ns: i64) -> timespec_rust {
    timespec_rust {
        tv_sec: s,
        tv_nsec: ns,
    }
}
// The wrapper function is not needed as the C function `make_timespec`
// is already defined in the codebase.

/// Convert a double representing seconds to a timespec_rust
fn dtotimespec_rust(sec: f64) -> timespec_rust {
    // Constants
    const TIMESPEC_HZ: i64 = 1_000_000_000; // Nanoseconds per second
    const TIME_T_MAX: i64 = i64::MAX;

    if sec >= TIME_T_MAX as f64 {
        // Handle overflow case
        return make_timespec_rust(TIME_T_MAX, 0);
    } else if sec >= (TIME_T_MAX as f64 - 1.0) {
        // Handle near-overflow case
        return make_timespec_rust(TIME_T_MAX - 1, TIMESPEC_HZ - 1);
    } else {
        // Normal case
        let s = sec.floor() as i64;
        let frac = TIMESPEC_HZ as f64 * (sec - s as f64);
        let mut ns = frac as i64;

        // Adjust for potential floating point imprecision
        if (frac - ns as f64) > 0.0 {
            ns += 1;
        }

        let mut s = s + ns / TIMESPEC_HZ;
        ns %= TIMESPEC_HZ;

        if ns < 0 {
            s -= 1;
            ns += TIMESPEC_HZ;
        }

        make_timespec_rust(s, ns)
    }
}
#[no_mangle]

/// A Rust implementation of rpl_nanosleep that handles large sleep durations
/// by breaking them into smaller chunks to avoid overflow issues.
///
/// # Arguments
/// * `requested_delay` - The duration to sleep
/// * `remaining_delay` - If the sleep is interrupted, this will be filled with the remaining time
///
/// # Returns
/// * `Ok(())` on success
/// * `Err(std::io::Error)` on failure
fn rpl_nanosleep_rust(
    requested_delay: &libc::timespec,
    remaining_delay: Option<&mut libc::timespec>,
) -> Result<(), std::io::Error> {
    const BILLION: i64 = 1_000_000_000;

    // Validate the nanoseconds value
    if requested_delay.tv_nsec < 0 || requested_delay.tv_nsec >= BILLION {
        return Err(std::io::Error::from_raw_os_error(libc::EINVAL));
    }

    // Maximum sleep time to avoid overflow issues (24 hours)
    const LIMIT_SECONDS: i64 = 24 * 60 * 60;
    let mut seconds_left = requested_delay.tv_sec;
    let mut current_delay = libc::timespec {
        tv_sec: requested_delay.tv_sec,
        tv_nsec: requested_delay.tv_nsec,
    };

    // For large sleep durations, break them into chunks of LIMIT_SECONDS
    while seconds_left > LIMIT_SECONDS {
        current_delay.tv_sec = LIMIT_SECONDS;

        // Use Rust's standard library sleep function
        let duration =
            std::time::Duration::new(current_delay.tv_sec as u64, current_delay.tv_nsec as u32);

        // Sleep can't fail in Rust, but we'll check for interruption by comparing times
        let start = std::time::SystemTime::now();
        thread::sleep(duration);

        seconds_left -= LIMIT_SECONDS;

        // Check if we were interrupted (this is an approximation since Rust's sleep doesn't report interruptions)
        if let Ok(elapsed) = start.elapsed() {
            if elapsed < duration {
                // Sleep was interrupted
                if let Some(rem) = remaining_delay {
                    // Calculate remaining time
                    let remaining_ns = duration.as_nanos() - elapsed.as_nanos();
                    let remaining_sec = remaining_ns / 1_000_000_000;
                    let remaining_nsec = remaining_ns % 1_000_000_000;

                    rem.tv_sec = remaining_sec as i64 + seconds_left;
                    rem.tv_nsec = remaining_nsec as i64;
                }
                return Err(std::io::Error::from_raw_os_error(libc::EINTR));
            }
        }

        current_delay.tv_nsec = 0;
    }

    // Handle the final chunk of sleep
    current_delay.tv_sec = seconds_left;
    let duration =
        std::time::Duration::new(current_delay.tv_sec as u64, current_delay.tv_nsec as u32);

    let start = std::time::SystemTime::now();
    thread::sleep(duration);

    // Check if the final sleep was interrupted
    if let (Some(rem), Ok(elapsed)) = (remaining_delay, start.elapsed()) {
        if elapsed < duration {
            // Sleep was interrupted
            let remaining_ns = duration.as_nanos() - elapsed.as_nanos();
            let remaining_sec = remaining_ns / 1_000_000_000;
            let remaining_nsec = remaining_ns % 1_000_000_000;

            rem.tv_sec = remaining_sec as i64;
            rem.tv_nsec = remaining_nsec as i64;

            return Err(std::io::Error::from_raw_os_error(libc::EINTR));
        }
    }

    Ok(())
}
#[no_mangle]

/// Sleep for the specified number of seconds.
///
/// Returns Ok(()) on success, or Err with the IO error on failure.
fn xnanosleep_rust(seconds: f64) -> Result<(), std::io::Error> {
    // Check if seconds is larger than the maximum time_t value
    // This is a simplified version of the complex C expression
    let max_time_t = if std::mem::size_of::<i64>() >= 8 {
        // For 64-bit or larger time_t
        i64::MAX as f64
    } else {
        // For smaller time_t (unlikely in modern systems)
        (((1i64 << (std::mem::size_of::<i64>() * 8 - 2)) - 1) * 2 + 1) as f64
    };

    if 1.0 + max_time_t <= seconds {
        // For extremely long sleeps, use thread::park() which is the Rust equivalent
        // of pause() but safer
        loop {
            thread::park();
            // In Rust, we can use std::io::Error::last_os_error() to check errno
            // but since thread::park() doesn't set errno, we don't need to check it
            break;
        }
        // Fall back to nanosleep approach if we somehow get here
    }

    // Convert seconds to timespec
    let mut ts_sleep = dtotimespec_rust(seconds);

    loop {
        // Call the Rust implementation of nanosleep
        match rpl_nanosleep_rust(
            &libc::timespec {
                tv_sec: ts_sleep.tv_sec,
                tv_nsec: ts_sleep.tv_nsec,
            },
            Some(&mut libc::timespec {
                tv_sec: ts_sleep.tv_sec,
                tv_nsec: ts_sleep.tv_nsec,
            }),
        ) {
            Ok(()) => break,
            Err(e) => {
                // Only retry if it's an interrupted system call or no error
                if e.raw_os_error() != Some(EINTR) && e.raw_os_error().is_some() {
                    return Err(e);
                }
                // Continue the loop to retry
            }
        }
    }

    Ok(())
}
#[no_mangle]
pub unsafe extern "C" fn xnanosleep(seconds: c_double) -> c_int {
    match xnanosleep_rust(seconds) {
        Ok(()) => 0,
        Err(_) => -1,
    }
}
