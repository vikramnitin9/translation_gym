#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

use libc::{self, c_char, c_int};
use std::{
    env,
    ffi::{CStr, CString, OsStr},
    fs,
    io::{self, Write},
    os::unix::{ffi::OsStrExt, fs::MetadataExt},
    path::{Path, PathBuf},
    process, ptr,
    sync::{
        atomic::{AtomicI32, AtomicPtr, Ordering},
        Mutex, Once, OnceLock,
    },
    time::Duration,
};
include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
include!("main_func.rs");
pub struct ProgramNameWrapper {
    val: String,
}

impl ProgramNameWrapper {
    pub fn new() -> Self {
        // Read the current value of the global variable
        let current_ptr = unsafe {
            // Access the global variable
            program_name
        };

        // Convert to idiomatic Rust String
        let val = if current_ptr.is_null() {
            String::new()
        } else {
            unsafe { CStr::from_ptr(current_ptr).to_string_lossy().into_owned() }
        };

        Self { val }
    }

    
    pub fn set(&mut self, val: String) {
        // Update the struct field
        self.val = val.clone();

        // Convert to C string and update the global variable
        let c_string = CString::new(val).unwrap_or_default();

        // We need to leak the CString to ensure it lives for the program duration
        // This is necessary because we're setting a global variable
        let raw_ptr = c_string.into_raw();

        unsafe {
            // Update the global variable
            program_name = raw_ptr as *const _;
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
                    let c_str = CString::new(new_base).unwrap();
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
    program_name_wrapper.set(argv0_final.to_string());

    // On glibc systems, set program_invocation_name
    unsafe {
        let c_str = CString::new(argv0_final).unwrap();
        program_invocation_name = c_str.into_raw();
    }
}
#[no_mangle]


/// Returns the character set of the current locale.
///
/// This is a Rust implementation of the C function `locale_charset`.
/// It attempts to determine the character encoding of the current locale.
fn locale_charset_rust() -> String {
    // In Rust, we can use the standard library to get locale information
    // Since nl_langinfo is not directly available, we'll use environment variables
    // which is a common approach in Rust for getting locale information

    // Try to get the character set from environment variables
    let codeset = match env::var("LC_ALL")
        .or_else(|_| env::var("LC_CTYPE"))
        .or_else(|_| env::var("LANG"))
    {
        Ok(locale) => {
            // Extract charset from locale string (typically in format like "en_US.UTF-8")
            locale.split('.').nth(1).unwrap_or("").to_string()
        }
        Err(_) => String::new(),
    };

    // If we couldn't determine the charset or it's empty, default to ASCII
    // This matches the behavior of the original C function
    if codeset.is_empty() {
        "ASCII".to_string()
    } else {
        codeset
    }
}
#[no_mangle]
pub unsafe extern "C" fn locale_charset() -> *const c_char {
    // Call the Rust implementation
    let charset = locale_charset_rust();

    // Convert the Rust String to a C string and leak it
    // This is necessary because we need to return a pointer that will remain valid
    // Note: This creates a memory leak, but it matches the behavior of the original C function
    // which returns a pointer to a static buffer
    let c_str = CString::new(charset).unwrap_or(CString::new("ASCII").unwrap());
    c_str.into_raw() as *const c_char
}

fn c_tolower_rust(c: i32) -> i32 {
    // Check if the character is an uppercase ASCII letter
    if c >= 'A' as i32 && c <= 'Z' as i32 {
        // Convert to lowercase by adding the difference between 'a' and 'A'
        c + ('a' as i32 - 'A' as i32)
    } else {
        // Return the character unchanged if it's not an uppercase ASCII letter
        c
    }
}
#[no_mangle]

fn c_strcasecmp_rust(s1: &str, s2: &str) -> i32 {
    // If the pointers are the same, the strings are identical
    if s1.as_ptr() == s2.as_ptr() {
        return 0;
    }

    // Iterate through both strings character by character
    let mut iter1 = s1.bytes();
    let mut iter2 = s2.bytes();

    loop {
        // Get the next character from each string and convert to lowercase
        let c1 = match iter1.next() {
            Some(c) => c_tolower_rust(c as i32),
            None => 0, // End of string
        };

        let c2 = match iter2.next() {
            Some(c) => c_tolower_rust(c as i32),
            None => 0, // End of string
        };

        // If we've reached the end of the first string, break
        if c1 == 0 {
            break;
        }

        // If characters differ, break
        if c1 != c2 {
            break;
        }
    }

    // Get the final characters that were compared
    let c1 = match iter1.next() {
        Some(c) => c_tolower_rust(c as i32),
        None => 0,
    };

    let c2 = match iter2.next() {
        Some(c) => c_tolower_rust(c as i32),
        None => 0,
    };

    // Handle potential overflow as in the original C code
    if (127 * 2 + 1) <= 2147483647 {
        c1 - c2
    } else {
        // This is the equivalent of ((c1 > c2) - (c1 < c2))
        match c1.cmp(&c2) {
            std::cmp::Ordering::Greater => 1,
            std::cmp::Ordering::Less => -1,
            std::cmp::Ordering::Equal => 0,
        }
    }
}
#[no_mangle]
pub unsafe extern "C" fn c_strcasecmp(s1: *const c_char, s2: *const c_char) -> c_int {
    // Check for null pointers
    if s1.is_null() || s2.is_null() {
        return 0;
    }

    // If the pointers are the same, the strings are identical
    if s1 == s2 {
        return 0;
    }

    // Convert C strings to Rust strings
    let rust_s1 = match CStr::from_ptr(s1).to_str() {
        Ok(s) => s,
        Err(_) => return 0, // Invalid UTF-8
    };

    let rust_s2 = match CStr::from_ptr(s2).to_str() {
        Ok(s) => s,
        Err(_) => return 0, // Invalid UTF-8
    };

    // Call the Rust implementation
    c_strcasecmp_rust(rust_s1, rust_s2)
}

fn proper_name_lite_rust(name_ascii: &str, name_utf8: &str) -> String {
    // Get the translation using gettext
    let translation = unsafe {
        let c_name_ascii = CString::new(name_ascii).unwrap();
        let result_ptr = gettext(c_name_ascii.as_ptr());
        if result_ptr.is_null() {
            name_ascii.to_string()
        } else {
            CStr::from_ptr(result_ptr).to_string_lossy().into_owned()
        }
    };

    // Check if translation is different from name_ascii
    if translation != name_ascii {
        translation
    } else if c_strcasecmp_rust(&locale_charset_rust(), "UTF-8") == 0 {
        name_utf8.to_string()
    } else {
        name_ascii.to_string()
    }
}
#[no_mangle]

pub struct VersionEtcCopyrightWrapper {
    val: String,
}

impl VersionEtcCopyrightWrapper {
    pub fn new() -> Self {
        Self {
            val: Self::get_global_value(),
        }
    }

    pub fn get(&self) -> String {
        Self::get_global_value()
    }

    
    // Helper method to read from the global variable
    fn get_global_value() -> String {
        unsafe {
            // Since version_etc_copyright is a zero-sized array, it's likely
            // meant to be a pointer to a null-terminated string.
            // We'll treat it as a pointer to the first element
            let ptr = &version_etc_copyright as *const _ as *const c_char;

            // If the pointer is valid and points to a null-terminated string
            if !ptr.is_null() {
                CStr::from_ptr(ptr).to_string_lossy().into_owned()
            } else {
                String::new()
            }
        }
    }
}

/// Outputs version and copyright information to the given writer.
///
/// # Arguments
///
/// * `stream` - The writer to output to
/// * `command_name` - Optional name of the command
/// * `package` - Name of the package
/// * `version` - Version string
/// * `authors` - List of authors
/// * `copyright_wrapper` - Copyright text wrapper
pub fn version_etc_arn_rust<W: Write>(
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
    /* TRANSLATORS: Translate "(C)" to the copyright symbol
    (C-in-a-circle), if this symbol is available in the user's
    locale.  Otherwise, do not translate "(C)"; leave it as-is.  */
    let copyright_text = copyright_wrapper.get();
    let c_symbol = unsafe {
        let c_str = CString::new("(C)").unwrap();
        let result = gettext(c_str.as_ptr());
        if result.is_null() {
            "(C)".to_string()
        } else {
            CStr::from_ptr(result).to_string_lossy().into_owned()
        }
    };

    writeln!(stream, "{}", copyright_text.replace("{}", &c_symbol))?;
    writeln!(stream)?;

    /* TRANSLATORS: The %s placeholder is the web address of the GPL license. */
    let license_text = unsafe {
        let c_str = CString::new(
            "License GPLv3+: GNU GPL version 3 or later <%s>.\n\
             This is free software: you are free to change and redistribute it.\n\
             There is NO WARRANTY, to the extent permitted by law.",
        )
        .unwrap();
        let result = gettext(c_str.as_ptr());
        if result.is_null() {
            "License GPLv3+: GNU GPL version 3 or later <{}>.\n\
             This is free software: you are free to change and redistribute it.\n\
             There is NO WARRANTY, to the extent permitted by law."
                .to_string()
        } else {
            CStr::from_ptr(result).to_string_lossy().into_owned()
        }
    };

    writeln!(
        stream,
        "{}",
        license_text.replace("%s", "https://gnu.org/licenses/gpl.html")
    )?;

    writeln!(stream)?;

    // Print author information based on the number of authors
    match authors.len() {
        0 => {
            // No authors are given. The caller should output authorship
            // info after calling this function.
        }
        1 => {
            /* TRANSLATORS: %s denotes an author name. */
            let text = unsafe {
                let c_str = CString::new("Written by %s.\n").unwrap();
                let result = gettext(c_str.as_ptr());
                if result.is_null() {
                    "Written by %s.\n".to_string()
                } else {
                    CStr::from_ptr(result).to_string_lossy().into_owned()
                }
            };
            write!(stream, "{}", text.replace("%s", authors[0]))?;
        }
        2 => {
            /* TRANSLATORS: Each %s denotes an author name. */
            let text = unsafe {
                let c_str = CString::new("Written by %s and %s.\n").unwrap();
                let result = gettext(c_str.as_ptr());
                if result.is_null() {
                    "Written by %s and %s.\n".to_string()
                } else {
                    CStr::from_ptr(result).to_string_lossy().into_owned()
                }
            };
            let text = text.replacen("%s", authors[0], 1);
            let text = text.replacen("%s", authors[1], 1);
            write!(stream, "{}", text)?;
        }
        3 => {
            /* TRANSLATORS: Each %s denotes an author name. */
            let text = unsafe {
                let c_str = CString::new("Written by %s, %s, and %s.\n").unwrap();
                let result = gettext(c_str.as_ptr());
                if result.is_null() {
                    "Written by %s, %s, and %s.\n".to_string()
                } else {
                    CStr::from_ptr(result).to_string_lossy().into_owned()
                }
            };
            let text = text.replacen("%s", authors[0], 1);
            let text = text.replacen("%s", authors[1], 1);
            let text = text.replacen("%s", authors[2], 1);
            write!(stream, "{}", text)?;
        }
        4 => {
            /* TRANSLATORS: Each %s denotes an author name.
            You can use line breaks, estimating that each author name occupies
            ca. 16 screen columns and that a screen line has ca. 80 columns. */
            let text = unsafe {
                let c_str = CString::new("Written by %s, %s, %s,\nand %s.\n").unwrap();
                let result = gettext(c_str.as_ptr());
                if result.is_null() {
                    "Written by %s, %s, %s,\nand %s.\n".to_string()
                } else {
                    CStr::from_ptr(result).to_string_lossy().into_owned()
                }
            };
            let text = text.replacen("%s", authors[0], 1);
            let text = text.replacen("%s", authors[1], 1);
            let text = text.replacen("%s", authors[2], 1);
            let text = text.replacen("%s", authors[3], 1);
            write!(stream, "{}", text)?;
        }
        5 => {
            /* TRANSLATORS: Each %s denotes an author name.
            You can use line breaks, estimating that each author name occupies
            ca. 16 screen columns and that a screen line has ca. 80 columns. */
            let text = unsafe {
                let c_str = CString::new("Written by %s, %s, %s,\n%s, and %s.\n").unwrap();
                let result = gettext(c_str.as_ptr());
                if result.is_null() {
                    "Written by %s, %s, %s,\n%s, and %s.\n".to_string()
                } else {
                    CStr::from_ptr(result).to_string_lossy().into_owned()
                }
            };
            let text = text.replacen("%s", authors[0], 1);
            let text = text.replacen("%s", authors[1], 1);
            let text = text.replacen("%s", authors[2], 1);
            let text = text.replacen("%s", authors[3], 1);
            let text = text.replacen("%s", authors[4], 1);
            write!(stream, "{}", text)?;
        }
        6 => {
            /* TRANSLATORS: Each %s denotes an author name.
            You can use line breaks, estimating that each author name occupies
            ca. 16 screen columns and that a screen line has ca. 80 columns. */
            let text = unsafe {
                let c_str = CString::new("Written by %s, %s, %s,\n%s, %s, and %s.\n").unwrap();
                let result = gettext(c_str.as_ptr());
                if result.is_null() {
                    "Written by %s, %s, %s,\n%s, %s, and %s.\n".to_string()
                } else {
                    CStr::from_ptr(result).to_string_lossy().into_owned()
                }
            };
            let text = text.replacen("%s", authors[0], 1);
            let text = text.replacen("%s", authors[1], 1);
            let text = text.replacen("%s", authors[2], 1);
            let text = text.replacen("%s", authors[3], 1);
            let text = text.replacen("%s", authors[4], 1);
            let text = text.replacen("%s", authors[5], 1);
            write!(stream, "{}", text)?;
        }
        7 => {
            /* TRANSLATORS: Each %s denotes an author name.
            You can use line breaks, estimating that each author name occupies
            ca. 16 screen columns and that a screen line has ca. 80 columns. */
            let text = unsafe {
                let c_str = CString::new("Written by %s, %s, %s,\n%s, %s, %s, and %s.\n").unwrap();
                let result = gettext(c_str.as_ptr());
                if result.is_null() {
                    "Written by %s, %s, %s,\n%s, %s, %s, and %s.\n".to_string()
                } else {
                    CStr::from_ptr(result).to_string_lossy().into_owned()
                }
            };
            let text = text.replacen("%s", authors[0], 1);
            let text = text.replacen("%s", authors[1], 1);
            let text = text.replacen("%s", authors[2], 1);
            let text = text.replacen("%s", authors[3], 1);
            let text = text.replacen("%s", authors[4], 1);
            let text = text.replacen("%s", authors[5], 1);
            let text = text.replacen("%s", authors[6], 1);
            write!(stream, "{}", text)?;
        }
        8 => {
            /* TRANSLATORS: Each %s denotes an author name.
            You can use line breaks, estimating that each author name occupies
            ca. 16 screen columns and that a screen line has ca. 80 columns. */
            let text = unsafe {
                let c_str =
                    CString::new("Written by %s, %s, %s,\n%s, %s, %s, %s,\nand %s.\n").unwrap();
                let result = gettext(c_str.as_ptr());
                if result.is_null() {
                    "Written by %s, %s, %s,\n%s, %s, %s, %s,\nand %s.\n".to_string()
                } else {
                    CStr::from_ptr(result).to_string_lossy().into_owned()
                }
            };
            let text = text.replacen("%s", authors[0], 1);
            let text = text.replacen("%s", authors[1], 1);
            let text = text.replacen("%s", authors[2], 1);
            let text = text.replacen("%s", authors[3], 1);
            let text = text.replacen("%s", authors[4], 1);
            let text = text.replacen("%s", authors[5], 1);
            let text = text.replacen("%s", authors[6], 1);
            let text = text.replacen("%s", authors[7], 1);
            write!(stream, "{}", text)?;
        }
        9 => {
            /* TRANSLATORS: Each %s denotes an author name.
            You can use line breaks, estimating that each author name occupies
            ca. 16 screen columns and that a screen line has ca. 80 columns. */
            let text = unsafe {
                let c_str =
                    CString::new("Written by %s, %s, %s,\n%s, %s, %s, %s,\n%s, and %s.\n").unwrap();
                let result = gettext(c_str.as_ptr());
                if result.is_null() {
                    "Written by %s, %s, %s,\n%s, %s, %s, %s,\n%s, and %s.\n".to_string()
                } else {
                    CStr::from_ptr(result).to_string_lossy().into_owned()
                }
            };
            let text = text.replacen("%s", authors[0], 1);
            let text = text.replacen("%s", authors[1], 1);
            let text = text.replacen("%s", authors[2], 1);
            let text = text.replacen("%s", authors[3], 1);
            let text = text.replacen("%s", authors[4], 1);
            let text = text.replacen("%s", authors[5], 1);
            let text = text.replacen("%s", authors[6], 1);
            let text = text.replacen("%s", authors[7], 1);
            let text = text.replacen("%s", authors[8], 1);
            write!(stream, "{}", text)?;
        }
        _ => {
            /* 10 or more authors. Use an abbreviation, since the human reader
            will probably not want to read the entire list anyway. */
            /* TRANSLATORS: Each %s denotes an author name.
            You can use line breaks, estimating that each author name occupies
            ca. 16 screen columns and that a screen line has ca. 80 columns. */
            let text = unsafe {
                let c_str =
                    CString::new("Written by %s, %s, %s,\n%s, %s, %s, %s,\n%s, %s, and others.\n")
                        .unwrap();
                let result = gettext(c_str.as_ptr());
                if result.is_null() {
                    "Written by %s, %s, %s,\n%s, %s, %s, %s,\n%s, %s, and others.\n".to_string()
                } else {
                    CStr::from_ptr(result).to_string_lossy().into_owned()
                }
            };
            let text = text.replacen("%s", authors[0], 1);
            let text = text.replacen("%s", authors[1], 1);
            let text = text.replacen("%s", authors[2], 1);
            let text = text.replacen("%s", authors[3], 1);
            let text = text.replacen("%s", authors[4], 1);
            let text = text.replacen("%s", authors[5], 1);
            let text = text.replacen("%s", authors[6], 1);
            let text = text.replacen("%s", authors[7], 1);
            let text = text.replacen("%s", authors[8], 1);
            write!(stream, "{}", text)?;
        }
    }

    Ok(())
}

/// Helper function to get a string from a C string pointer
fn c_str_to_option_str(c_str: *const c_char) -> Option<&'static str> {
    if c_str.is_null() {
        None
    } else {
        unsafe { Some(CStr::from_ptr(c_str).to_str().unwrap_or("")) }
    }
}

/// Helper function to convert a slice of C string pointers to a vector of Rust string slices
fn c_str_array_to_str_vec(c_strs: *const *const c_char, n: usize) -> Vec<&'static str> {
    let mut result = Vec::with_capacity(n);
    for i in 0..n {
        unsafe {
            let ptr = *c_strs.add(i);
            if !ptr.is_null() {
                if let Ok(s) = CStr::from_ptr(ptr).to_str() {
                    result.push(s);
                }
            }
        }
    }
    result
}
#[no_mangle]
pub unsafe extern "C" fn version_etc_arn(
    stream: *mut libc::FILE,
    command_name: *const c_char,
    package: *const c_char,
    version: *const c_char,
    authors: *const *const c_char,
    n_authors: libc::size_t,
) {
    // Create a wrapper for the FILE stream
    struct FileWriter(*mut libc::FILE);

    impl Write for FileWriter {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            let written =
                unsafe { libc::fwrite(buf.as_ptr() as *const libc::c_void, 1, buf.len(), self.0) };
            Ok(written)
        }

        fn flush(&mut self) -> io::Result<()> {
            let result = unsafe { libc::fflush(self.0) };
            if result == 0 {
                Ok(())
            } else {
                Err(io::Error::last_os_error())
            }
        }
    }

    // Safety: We're assuming the FILE pointer is valid
    let file_writer = FileWriter(stream);

    // Convert C strings to Rust strings
    let cmd_name = c_str_to_option_str(command_name);
    let pkg = match c_str_to_option_str(package) {
        Some(p) => p,
        None => return, // Package name is required
    };
    let ver = match c_str_to_option_str(version) {
        Some(v) => v,
        None => return, // Version is required
    };

    // Convert author array to Vec of &str
    let authors_vec = c_str_array_to_str_vec(authors, n_authors);
    let authors_slice: Vec<&str> = authors_vec.iter().map(|s| *s).collect();

    // Create the copyright wrapper
    let copyright_wrapper = VersionEtcCopyrightWrapper::new();

    // Call the Rust implementation
    let _ = version_etc_arn_rust(
        file_writer,
        cmd_name,
        pkg,
        ver,
        &authors_slice,
        &copyright_wrapper,
    );
}

/// Rust implementation that would be equivalent to version_etc_va
///
/// This function takes a list of authors and passes them to version_etc_arn_rust
// Since we can't implement C-variadic functions in stable Rust,
// we'll need to keep the original C implementation of version_etc_va.
// This is a stub that will be replaced by the actual C implementation.
#[no_mangle]
pub unsafe extern "C" fn version_etc_va(
    stream: *mut libc::FILE,
    command_name: *const c_char,
    package: *const c_char,
    version: *const c_char,
    // We can't represent variadic arguments in Rust without unstable features
    // This function will be implemented in C
) {
    // This is a stub that will be replaced by the actual C implementation
    // In the meantime, we'll implement a simplified version that doesn't use variadic arguments

    // Create an empty array of authors
    let authors: [*const c_char; 0] = [];

    // Call version_etc_arn directly
    version_etc_arn(stream, command_name, package, version, authors.as_ptr(), 0);
}

/// Represents a time value with separate seconds and nanoseconds components.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct timespec_rust {
    /// Seconds
    pub tv_sec: i64,
    /// Nanoseconds
    pub tv_nsec: i64,
}

impl timespec_rust {
    /// Creates a new timespec from seconds and nanoseconds
    
    /// Converts to a Rust standard Duration
    
    /// Creates from a Rust standard Duration
    pub fn from_duration(duration: Duration) -> Self {
        Self {
            tv_sec: duration.as_secs() as i64,
            tv_nsec: duration.subsec_nanos() as i64,
        }
    }
}

/// Rust equivalent of the C `struct stat`

/// Returns the logical current working directory from the PWD environment variable
/// if it's valid, otherwise returns None.
fn logical_getcwd_rust() -> Option<String> {
    // Get PWD from environment
    let wd = env::var("PWD").ok()?;

    // Textual validation - must start with '/'
    if !wd.starts_with('/') {
        return None;
    }

    // Check for problematic path components like "/." or "/.."
    let mut search_pos = 0;
    while let Some(pos) = wd[search_pos..].find("/.") {
        let absolute_pos = search_pos + pos;
        let after_dot = absolute_pos + 2;

        // Check if it's "/." at the end or "/./", or "/.." at the end or "/../"
        if after_dot >= wd.len()
            || wd.as_bytes()[after_dot] == b'/'
            || (after_dot + 1 < wd.len()
                && wd.as_bytes()[after_dot] == b'.'
                && (after_dot + 1 == wd.len() || wd.as_bytes()[after_dot + 1] == b'/'))
        {
            return None;
        }

        search_pos = absolute_pos + 1;
    }

    // System call validation - check if PWD and "." refer to the same directory
    let wd_metadata = fs::metadata(&wd).ok()?;
    let current_dir_metadata = fs::metadata(".").ok()?;

    // Compare device and inode to check if they're the same directory
    if wd_metadata.dev() == current_dir_metadata.dev()
        && wd_metadata.ino() == current_dir_metadata.ino()
    {
        Some(wd)
    } else {
        None
    }
}
#[no_mangle]

pub struct ExitFailureWrapper {
    val: i32,
}

impl ExitFailureWrapper {
    
    
    }

/// Gets the current working directory as a String.
/// Returns None if the current directory does not exist or is not accessible.
/// Panics if out of memory.
fn xgetcwd_rust() -> Option<String> {
    match env::current_dir() {
        Ok(path) => Some(path.to_string_lossy().into_owned()),
        Err(e) if e.kind() == io::ErrorKind::OutOfMemory => {
            // This is equivalent to xalloc_die() in the C code
            panic!("Memory exhausted");
        }
        Err(_) => None,
    }
}
#[no_mangle]

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Quoting_style_rust {
    /// Output names as-is (ls --quoting-style=literal).  Can result in
    /// embedded null bytes if QA_ELIDE_NULL_BYTES is not in effect.
    ///
    /// quotearg_buffer:
    /// "simple", "\0 \t\n'\"\033??/\\", "a:b"
    /// quotearg:
    /// "simple", " \t\n'\"\033??/\\", "a:b"
    /// quotearg_colon:
    /// "simple", " \t\n'\"\033??/\\", "a:b"
    Literal_quoting_style,

    /// Quote names for the shell if they contain shell metacharacters
    /// or would cause ambiguous output (ls --quoting-style=shell).
    /// Can result in embedded null bytes if QA_ELIDE_NULL_BYTES is not
    /// in effect.
    ///
    /// quotearg_buffer:
    /// "simple", "'\0 \t\n'\\''\"\033??/\\'", "a:b"
    /// quotearg:
    /// "simple", "' \t\n'\\''\"\033??/\\'", "a:b"
    /// quotearg_colon:
    /// "simple", "' \t\n'\\''\"\033??/\\'", "'a:b'"
    Shell_quoting_style,

    /// Quote names for the shell, even if they would normally not
    /// require quoting (ls --quoting-style=shell-always).  Can result
    /// in embedded null bytes if QA_ELIDE_NULL_BYTES is not in effect.
    /// Behaves like shell_quoting_style if QA_ELIDE_OUTER_QUOTES is in
    /// effect.
    ///
    /// quotearg_buffer:
    /// "'simple'", "'\0 \t\n'\\''\"\033??/\\'", "'a:b'"
    /// quotearg:
    /// "'simple'", "' \t\n'\\''\"\033??/\\'", "'a:b'"
    /// quotearg_colon:
    /// "'simple'", "' \t\n'\\''\"\033??/\\'", "'a:b'"
    Shell_always_quoting_style,

    /// Quote names for the shell if they contain shell metacharacters
    /// or other problematic characters (ls --quoting-style=shell-escape).
    /// Non printable characters are quoted using the $'...' syntax
    /// <https://www.gnu.org/software/bash/manual/html_node/ANSI_002dC-Quoting.html>,
    /// which originated in ksh93 and is widely supported by most shells,
    /// and proposed for inclusion in POSIX.
    ///
    /// quotearg_buffer:
    /// "simple", "''$'\\0'' '$'\\t\\n'\\''\"'$'\\033''??/\\'", "a:b"
    /// quotearg:
    /// "simple", "''$'\\0'' '$'\\t\\n'\\''\"'$'\\033''??/\\'", "a:b"
    /// quotearg_colon:
    /// "simple", "''$'\\0'' '$'\\t\\n'\\''\"'$'\\033''??/\\'", "'a:b'"
    Shell_escape_quoting_style,

    /// Quote names for the shell even if they would normally not
    /// require quoting (ls --quoting-style=shell-escape).
    /// Non printable characters are quoted using the $'...' syntax
    /// <https://www.gnu.org/software/bash/manual/html_node/ANSI_002dC-Quoting.html>,
    /// which originated in ksh93 and is widely supported by most shells,
    /// and proposed for inclusion in POSIX.  Behaves like
    /// shell_escape_quoting_style if QA_ELIDE_OUTER_QUOTES is in effect.
    ///
    /// quotearg_buffer:
    /// "simple", "''$'\\0'' '$'\\t\\n'\\''\"'$'\\033''??/\'", "a:b"
    /// quotearg:
    /// "simple", "''$'\\0'' '$'\\t\\n'\\''\"'$'\\033''??/\'", "a:b"
    /// quotearg_colon:
    /// "simple", "''$'\\0'' '$'\\t\\n'\\''\"'$'\\033''??/\'", "'a:b'"
    Shell_escape_always_quoting_style,

    /// Quote names as for a C language string (ls --quoting-style=c).
    /// Behaves like c_maybe_quoting_style if QA_ELIDE_OUTER_QUOTES is
    /// in effect.  Split into consecutive strings if
    /// QA_SPLIT_TRIGRAPHS.
    ///
    /// quotearg_buffer:
    /// "\"simple\"", "\"\\0 \\t\\n'\\\"\\033??/\\\\\"", "\"a:b\""
    /// quotearg:
    /// "\"simple\"", "\"\\0 \\t\\n'\\\"\\033??/\\\\\"", "\"a:b\""
    /// quotearg_colon:
    /// "\"simple\"", "\"\\0 \\t\\n'\\\"\\033??/\\\\\"", "\"a\\:b\""
    C_quoting_style,

    /// Like c_quoting_style except omit the surrounding double-quote
    /// characters if no quoted characters are encountered.
    ///
    /// quotearg_buffer:
    /// "simple", "\"\\0 \\t\\n'\\\"\\033??/\\\\\"", "a:b"
    /// quotearg:
    /// "simple", "\"\\0 \\t\\n'\\\"\\033??/\\\\\"", "a:b"
    /// quotearg_colon:
    /// "simple", "\"\\0 \\t\\n'\\\"\\033??/\\\\\"", "\"a:b\""
    C_maybe_quoting_style,

    /// Like c_quoting_style except always omit the surrounding
    /// double-quote characters and ignore QA_SPLIT_TRIGRAPHS
    /// (ls --quoting-style=escape).
    ///
    /// quotearg_buffer:
    /// "simple", "\\0 \\t\\n'\"\\033??/\\\\", "a:b"
    /// quotearg:
    /// "simple", "\\0 \\t\\n'\"\\033??/\\\\", "a:b"
    /// quotearg_colon:
    /// "simple", "\\0 \\t\\n'\"\\033??/\\\\", "a\\:b"
    Escape_quoting_style,

    /// Like clocale_quoting_style, but use single quotes in the
    /// default C locale or if the program does not use gettext
    /// (ls --quoting-style=locale).  For UTF-8 locales, quote
    /// characters will use Unicode.
    ///
    /// LC_MESSAGES=C
    /// quotearg_buffer:
    /// "'simple'", "'\\0 \\t\\n\\'\"\\033??/\\\\'", "'a:b'"
    /// quotearg:
    /// "'simple'", "'\\0 \\t\\n\\'\"\\033??/\\\\'", "'a:b'"
    /// quotearg_colon:
    /// "'simple'", "'\\0 \\t\\n\\'\"\\033??/\\\\'", "'a\\:b'"
    ///
    /// LC_MESSAGES=pt_PT.utf8
    /// quotearg_buffer:
    /// "\302\253simple\302\273",
    /// "\302\253\\0 \\t\\n'\"\\033??/\\\\\302\253", "\302\253a:b\302\273"
    /// quotearg:
    /// "\302\253simple\302\273",
    /// "\302\253\\0 \\t\\n'\"\\033??/\\\\\302\253", "\302\253a:b\302\273"
    /// quotearg_colon:
    /// "\302\253simple\302\273",
    /// "\302\253\\0 \\t\\n'\"\\033??/\\\\\302\253", "\302\253a\\:b\302\273"
    Locale_quoting_style,

    /// Like c_quoting_style except use quotation marks appropriate for
    /// the locale and ignore QA_SPLIT_TRIGRAPHS
    /// (ls --quoting-style=clocale).
    ///
    /// LC_MESSAGES=C
    /// quotearg_buffer:
    /// "\"simple\"", "\"\\0 \\t\\n'\\\"\\033??/\\\\\"", "\"a:b\""
    /// quotearg:
    /// "\"simple\"", "\"\\0 \\t\\n'\\\"\\033??/\\\\\"", "\"a:b\""
    /// quotearg_colon:
    /// "\"simple\"", "\"\\0 \\t\\n'\\\"\\033??/\\\\\"", "\"a\\:b\""
    ///
    /// LC_MESSAGES=pt_PT.utf8
    /// quotearg_buffer:
    /// "\302\253simple\302\273",
    /// "\302\253\\0 \\t\\n'\"\\033??/\\\\\302\253", "\302\253a:b\302\273"
    /// quotearg:
    /// "\302\253simple\302\273",
    /// "\302\253\\0 \\t\\n'\"\\033??/\\\\\302\253", "\302\253a:b\302\273"
    /// quotearg_colon:
    /// "\302\253simple\302\273",
    /// "\302\253\\0 \\t\\n'\"\\033??/\\\\\302\253", "\302\253a\\:b\302\273"
    Clocale_quoting_style,

    /// Like clocale_quoting_style except use the custom quotation marks
    /// set by set_custom_quoting.  If custom quotation marks are not
    /// set, the behavior is undefined.
    ///
    /// left_quote = right_quote = "'"
    /// quotearg_buffer:
    /// "'simple'", "'\\0 \\t\\n\\'\"\\033??/\\\\'", "'a:b'"
    /// quotearg:
    /// "'simple'", "'\\0 \\t\\n\\'\"\\033??/\\\\'", "'a:b'"
    /// quotearg_colon:
    /// "'simple'", "'\\0 \\t\\n\\'\"\\033??/\\\\'", "'a\\:b'"
    ///
    /// left_quote = "(" and right_quote = ")"
    /// quotearg_buffer:
    /// "(simple)", "(\\0 \\t\\n'\"\\033??/\\\\)", "(a:b)"
    /// quotearg:
    /// "(simple)", "(\\0 \\t\\n'\"\\033??/\\\\)", "(a:b)"
    /// quotearg_colon:
    /// "(simple)", "(\\0 \\t\\n'\"\\033??/\\\\)", "(a\\:b)"
    ///
    /// left_quote = ":" and right_quote = " "
    /// quotearg_buffer:
    /// ":simple ", ":\\0\\ \\t\\n'\"\\033??/\\\\ ", ":a:b "
    /// quotearg:
    /// ":simple ", ":\\0\\ \\t\\n'\"\\033??/\\\\ ", ":a:b "
    /// quotearg_colon:
    /// ":simple ", ":\\0\\ \\t\\n'\"\\033??/\\\\ ", ":a\\:b "
    ///
    /// left_quote = "\"'" and right_quote = "'\""
    /// Notice that this is treated as a single level of quotes or two
    /// levels where the outer quote need not be escaped within the inner
    /// quotes.  For two levels where the outer quote must be escaped
    /// within the inner quotes, you must use separate quotearg
    /// invocations.
    /// quotearg_buffer:
    /// "\"'simple'\"", "\"'\\0 \\t\\n\\'\"\\033??/\\\\'\"", "\"'a:b'\""
    /// quotearg:
    /// "\"'simple'\"", "\"'\\0 \\t\\n\\'\"\\033??/\\\\'\"", "\"'a:b'\""
    /// quotearg_colon:
    /// "\"'simple'\"", "\"'\\0 \\t\\n\\'\"\\033??/\\\\'\"", "\"'a\\:b'\""
    Custom_quoting_style,
}


#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(i32)]
pub enum _rust {
    // Index values
    __ISwupper = 0,  // UPPERCASE
    __ISwlower = 1,  // lowercase
    __ISwalpha = 2,  // Alphabetic
    __ISwdigit = 3,  // Numeric
    __ISwxdigit = 4, // Hexadecimal numeric
    __ISwspace = 5,  // Whitespace
    __ISwprint = 6,  // Printing
    __ISwgraph = 7,  // Graphical
    __ISwblank = 8,  // Blank (usually SPC and TAB)
    __ISwcntrl = 9,  // Control character
    __ISwpunct = 10, // Punctuation
    __ISwalnum = 11, // Alphanumeric

    // Bitmask values - calculated based on the index values
    _ISwupper = (1 << 0) << 24,  // UPPERCASE
    _ISwlower = (1 << 1) << 24,  // lowercase
    _ISwalpha = (1 << 2) << 24,  // Alphabetic
    _ISwdigit = (1 << 3) << 24,  // Numeric
    _ISwxdigit = (1 << 4) << 24, // Hexadecimal numeric
    _ISwspace = (1 << 5) << 24,  // Whitespace
    _ISwprint = (1 << 6) << 24,  // Printing
    _ISwgraph = (1 << 7) << 24,  // Graphical
    _ISwblank = (1 << 8) << 8,   // Blank (usually SPC and TAB)
    _ISwcntrl = (1 << 9) << 8,   // Control character
    _ISwpunct = (1 << 10) << 8,  // Punctuation
    _ISwalnum = (1 << 11) << 8,  // Alphanumeric
}

pub struct QuotingOptions {
    // Fields representing the quoting options in an idiomatic way
    pub style: QuotingStyle,
    pub flags: u32,
    pub quote_these_too: Option<Vec<char>>,
    pub left_quote: String,
    pub right_quote: String,
}

pub enum QuotingStyle {
    Literal,
    Shell,
    ShellAlways,
    C,
    CLocale,
    Escape,
    Custom,
}

// Global static using thread-safe access pattern


pub struct QuoteQuotingOptionsWrapper {
    val: QuotingOptions,
}

impl QuoteQuotingOptionsWrapper {
    
    
    }

// Implement Clone for QuotingOptions
impl Clone for QuotingOptions {
    fn clone(&self) -> Self {
        Self {
            style: self.style.clone(),
            flags: self.flags,
            quote_these_too: self.quote_these_too.clone(),
            left_quote: self.left_quote.clone(),
            right_quote: self.right_quote.clone(),
        }
    }
}

// Implement Clone for QuotingStyle
impl Clone for QuotingStyle {
    fn clone(&self) -> Self {
        match self {
            QuotingStyle::Literal => QuotingStyle::Literal,
            QuotingStyle::Shell => QuotingStyle::Shell,
            QuotingStyle::ShellAlways => QuotingStyle::ShellAlways,
            QuotingStyle::C => QuotingStyle::C,
            QuotingStyle::CLocale => QuotingStyle::CLocale,
            QuotingStyle::Escape => QuotingStyle::Escape,
            QuotingStyle::Custom => QuotingStyle::Custom,
        }
    }
}

#[repr(C)]

/// Represents a device and inode pair
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct dev_ino_rust {
    pub st_ino: u64, // inode number
    pub st_dev: u64, // device ID
}

pub struct VersionWrapper {
    val: String,
}

impl VersionWrapper {
    pub fn new() -> Self {
        let version_str = unsafe {
            if Version.is_null() {
                String::new()
            } else {
                CStr::from_ptr(Version).to_string_lossy().into_owned()
            }
        };

        Self { val: version_str }
    }

    pub fn get(&self) -> String {
        unsafe {
            if Version.is_null() {
                String::new()
            } else {
                CStr::from_ptr(Version).to_string_lossy().into_owned()
            }
        }
    }

    }

pub struct LongoptsWrapper {
    val: Vec<CommandOption>,
}

impl LongoptsWrapper {
    pub fn new() -> Self {
        // Read the global variable and convert to idiomatic type
        let options = unsafe {
            // Convert the static array to a Vec
            // Since the array has 0 length, we're creating an empty Vec
            Vec::new()
        };

        Self { val: options }
    }

    
    }

// An idiomatic Rust representation of the option struct
// This assumes the original C struct had fields like name, has_arg, flag, and val
pub struct CommandOption {
    pub name: String,
    pub has_arg: bool,
    pub flag: Option<i32>,
    pub val: char,
}

fn main_0_rust(
    args: Vec<String>,
    program_name_wrapper: &mut ProgramNameWrapper,
    version_wrapper: &VersionWrapper,
    longopts_wrapper: &LongoptsWrapper,
) -> i32 {
    // Check if POSIXLY_CORRECT is set in the environment
    let mut logical = env::var("POSIXLY_CORRECT").is_ok();

    // Set program name
    set_program_name_rust(&args[0], program_name_wrapper);

    // Set locale and text domain
    // Using unsafe blocks for C FFI calls
    unsafe {
        let c_locale = CString::new("").unwrap();
        setlocale(6, c_locale.as_ptr());

        let c_domain = CString::new("coreutils").unwrap();
        let c_dir = CString::new("/usr/local/share/locale").unwrap();
        bindtextdomain(c_domain.as_ptr(), c_dir.as_ptr());
        textdomain(c_domain.as_ptr());

        // Register close_stdout to be called on exit
        atexit(Some(close_stdout));
    }

    // Parse command line arguments manually
    let mut i = 1;
    while i < args.len() {
        let arg = &args[i];

        if !arg.starts_with('-') || arg == "-" {
            break;
        }

        if arg == "--" {
            i += 1;
            break;
        }

        if arg == "--help" {
            unsafe {
                usage(0);
            }
            return 0;
        } else if arg == "--version" {
            unsafe {
                let c_command = CString::new("pwd").unwrap();
                let c_package = CString::new("GNU coreutils").unwrap();
                let c_version = CString::new(version_wrapper.get()).unwrap();
                let c_author =
                    CString::new(proper_name_lite_rust("Jim Meyering", "Jim Meyering")).unwrap();

                version_etc(
                    stdout,
                    c_command.as_ptr(),
                    c_package.as_ptr(),
                    c_version.as_ptr(),
                    c_author.as_ptr(),
                    ptr::null::<libc::c_char>(),
                );
            }
            process::exit(0);
        } else if arg == "-L" {
            logical = true;
        } else if arg == "-P" {
            logical = false;
        } else if arg.starts_with("--") {
            // Handle long options
            if arg == "--logical" {
                logical = true;
            } else if arg == "--physical" {
                logical = false;
            } else {
                unsafe {
                    usage(1);
                }
                return 1;
            }
        } else if arg.starts_with('-') {
            // Handle short options
            for c in arg[1..].chars() {
                match c {
                    'L' => logical = true,
                    'P' => logical = false,
                    _ => {
                        unsafe {
                            usage(1);
                        }
                        return 1;
                    }
                }
            }
        }

        i += 1;
    }

    // Check for non-option arguments and ignore them with a warning
    if i < args.len() {
        unsafe {
            let c_message = CString::new("ignoring non-option arguments").unwrap();
            error(0, 0, c_message.as_ptr());
        }
    }

    // Try to get the current working directory
    if logical {
        if let Some(wd) = logical_getcwd_rust() {
            println!("{}", wd);
            return 0;
        }
    }

    // If logical path failed or wasn't requested, try physical path
    if let Some(wd) = xgetcwd_rust() {
        println!("{}", wd);
    } else {
        // Fallback method if xgetcwd_rust fails
        // Since file_name_init, robust_getcwd, and file_name_free are not accessible,
        // we'll use a Rust alternative
        match std::env::current_dir() {
            Ok(path) => println!("{}", path.display()),
            Err(e) => {
                eprintln!("Failed to get current directory: {}", e);
                return 1;
            }
        }
    }

    0
}
#[no_mangle]
pub unsafe extern "C" fn main_0(argc: libc::c_int, argv: *mut *mut libc::c_char) -> libc::c_int {
    // Convert C arguments to Rust
    let mut args = Vec::new();
    for i in 0..argc {
        let arg_ptr = *argv.offset(i as isize);
        let c_str = CStr::from_ptr(arg_ptr);
        let arg = c_str.to_string_lossy().into_owned();
        args.push(arg);
    }

    // Create wrappers for global variables
    let mut program_name_wrapper = ProgramNameWrapper::new();
    let version_wrapper = VersionWrapper::new();
    let longopts_wrapper = LongoptsWrapper::new();

    // Call the Rust implementation
    main_0_rust(
        args,
        &mut program_name_wrapper,
        &version_wrapper,
        &longopts_wrapper,
    )
}
