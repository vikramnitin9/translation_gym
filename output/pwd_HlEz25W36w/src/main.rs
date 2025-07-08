#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

use libc::{self, c_char, c_int};
use std::{
    cell::UnsafeCell,
    env,
    ffi::{CStr, CString, OsStr},
    fs,
    io::{self, Write},
    os::{
        fd::FromRawFd,
        unix::{ffi::OsStrExt, fs::MetadataExt},
    },
    path::{Path, PathBuf},
    process, ptr,
    sync::{
        atomic::{AtomicBool, AtomicI32, AtomicPtr, Ordering},
        Mutex, Once,
    },
    time::Duration,
};
include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
include!("main_func.rs");
pub struct ProgramNameWrapper {
    val: Option<String>,
}

impl ProgramNameWrapper {
    pub fn new() -> Self {
        Self {
            val: Self::get_global(),
        }
    }

    
    pub fn set(&mut self, val: Option<String>) {
        self.val = val.clone();

        // Update the global variable
        unsafe {
            if let Some(s) = &val {
                // Convert String to C string and leak it (since we're setting a global)
                let c_str = std::ffi::CString::new(s.clone()).unwrap();
                program_name = c_str.into_raw();
            } else {
                program_name = ptr::null();
            }
        }
    }

    // Helper method to read from the global variable
    fn get_global() -> Option<String> {
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

/// Sets the program name based on the provided command-line argument.
///
/// This function handles special cases like executables created by libtool
/// that might have prefixes like "lt-" or be in special directories like ".libs/".
fn set_program_name_rust(argv0: &str, program_name_wrapper: &mut ProgramNameWrapper) {
    // Sanity check. POSIX requires the invoking process to pass a non-NULL argv[0].
    if argv0.is_empty() {
        // It's a bug in the invoking program. Help diagnosing it.
        eprintln!("A NULL argv[0] was passed through an exec system call.");
        process::abort();
    }

    // Find the last slash to determine the base name
    let base = match argv0.rfind('/') {
        Some(pos) => &argv0[pos + 1..],
        None => argv0,
    };

    // Check if the executable is in a .libs directory
    let argv0_final = if argv0.len() >= base.len() + 7 {
        let prefix_pos = argv0.len() - base.len() - 7;
        if &argv0[prefix_pos..prefix_pos + 7] == "/.libs/" {
            // It's in a .libs directory, so use just the base name
            // Check for "lt-" prefix in the base name
            if base.starts_with("lt-") {
                // On glibc systems, also update program_invocation_short_name
                unsafe {
                    program_invocation_short_name = base[3..].as_ptr() as *mut c_char;
                }
                &base[3..]
            } else {
                base
            }
        } else {
            // Not in a .libs directory
            argv0
        }
    } else {
        // Path too short to contain "/.libs/"
        argv0
    };

    // Set program_name
    program_name_wrapper.set(Some(argv0_final.to_string()));

    // On glibc systems, also update program_invocation_name
    unsafe {
        // Convert to CString and leak it to ensure it lives for the program duration
        // This is necessary because we're setting a global C variable
        let c_str = std::ffi::CString::new(argv0_final).unwrap();
        let ptr = c_str.into_raw();
        program_invocation_name = ptr;
    }
}
#[no_mangle]


/// Returns the character set of the current locale.
///
/// This is a Rust implementation of the C function `locale_charset`.
fn locale_charset_rust() -> String {
    // In Rust, we can use the locale_config crate or std::env to get locale information,
    // but for simplicity and to match the C function's behavior, we'll implement a basic version.

    // Get the character set from the system
    // Since nl_langinfo is not accessible, we'll use a Rust alternative
    let mut codeset = get_system_codeset();

    // Don't return an empty string
    if codeset.is_empty() {
        codeset = "ASCII".to_string();
    }

    codeset
}

/// Helper function to get the system's character encoding
/// This replaces the nl_langinfo(CODESET) call from the C version
fn get_system_codeset() -> String {
    // Try to determine the codeset from environment variables
    if let Ok(lang) = std::env::var("LC_ALL") {
        if !lang.is_empty() {
            return extract_codeset_from_locale(&lang);
        }
    }

    if let Ok(lang) = std::env::var("LC_CTYPE") {
        if !lang.is_empty() {
            return extract_codeset_from_locale(&lang);
        }
    }

    if let Ok(lang) = std::env::var("LANG") {
        if !lang.is_empty() {
            return extract_codeset_from_locale(&lang);
        }
    }

    // Default to empty string if we couldn't determine the codeset
    String::new()
}

/// Extract the character set from a locale string (e.g., "en_US.UTF-8" -> "UTF-8")
fn extract_codeset_from_locale(locale: &str) -> String {
    if let Some(dot_pos) = locale.find('.') {
        let codeset = &locale[dot_pos + 1..];
        if let Some(at_pos) = codeset.find('@') {
            return codeset[..at_pos].to_string();
        }
        return codeset.to_string();
    }

    // Many modern systems default to UTF-8
    if cfg!(target_os = "macos")
        || cfg!(target_os = "ios")
        || cfg!(target_os = "haiku")
        || cfg!(target_os = "beos")
    {
        return "UTF-8".to_string();
    }

    String::new()
}
#[no_mangle]
pub unsafe extern "C" fn locale_charset() -> *const c_char {
    // Call the Rust implementation
    let rust_result = locale_charset_rust();

    // Convert the Rust String to a C string
    // We need to leak the memory here because the C caller expects
    // a pointer that remains valid after this function returns
    let c_string = CString::new(rust_result).unwrap_or(CString::new("ASCII").unwrap());

    // This memory will leak, but that's expected for this FFI interface
    // The original C function returns a pointer to static data
    c_string.into_raw() as *const c_char
}

fn c_tolower_rust(c: char) -> char {
    match c {
        'A'..='Z' => ((c as u8) + b'a' - b'A') as char,
        _ => c,
    }
}
#[no_mangle]

fn c_strcasecmp_rust(s1: &str, s2: &str) -> i32 {
    // If the strings are the same reference, they're equal
    if s1.as_ptr() == s2.as_ptr() {
        return 0;
    }

    // Iterate through both strings character by character
    let mut iter1 = s1.chars();
    let mut iter2 = s2.chars();

    loop {
        // Get the next character from each string, converted to lowercase
        let c1_opt = iter1.next().map(|c| c_tolower_rust(c));
        let c2_opt = iter2.next().map(|c| c_tolower_rust(c));

        match (c1_opt, c2_opt) {
            // Both strings ended at the same time
            (None, None) => return 0,
            // First string is shorter
            (None, Some(_)) => return -1,
            // Second string is shorter
            (Some(_), None) => return 1,
            // Both have characters
            (Some(c1), Some(c2)) => {
                if c1 != c2 {
                    // Convert to u8 for comparison to match C behavior
                    let c1_u8 = c1 as u8;
                    let c2_u8 = c2 as u8;

                    // Check if we're on a machine where the difference might not fit in an int
                    if (127 * 2 + 1) <= 2147483647 {
                        return (c1_u8 as i32) - (c2_u8 as i32);
                    } else {
                        // Alternative comparison for machines where char and int are the same size
                        return if c1_u8 > c2_u8 { 1 } else { -1 };
                    }
                }

                // If we've reached a null character in the first string, we're done
                if c1 == '\0' {
                    break;
                }
            }
        }
    }

    0
}
#[no_mangle]
pub unsafe extern "C" fn c_strcasecmp(s1: *const c_char, s2: *const c_char) -> c_int {
    // Check for null pointers
    if s1.is_null() || s2.is_null() {
        return 0;
    }

    // If the pointers are the same, the strings are equal
    if s1 == s2 {
        return 0;
    }

    // Convert C strings to Rust strings
    let r_s1 = match CStr::from_ptr(s1).to_str() {
        Ok(s) => s,
        Err(_) => return 0, // Invalid UTF-8
    };

    let r_s2 = match CStr::from_ptr(s2).to_str() {
        Ok(s) => s,
        Err(_) => return 0, // Invalid UTF-8
    };

    // Call the Rust implementation
    c_strcasecmp_rust(r_s1, r_s2)
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

    
    // Helper method to read the global value
    fn get_global_value() -> String {
        unsafe {
            // Since version_etc_copyright is a zero-sized array, it's likely
            // meant to be a pointer to a null-terminated string.
            // We'll treat the address of the array as a pointer to a C string.
            let ptr = &version_etc_copyright as *const _ as *const c_char;
            if ptr.is_null() {
                String::new()
            } else {
                CStr::from_ptr(ptr).to_string_lossy().into_owned()
            }
        }
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
/// * `authors` - List of author names
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
    // TRANSLATORS: Translate "(C)" to the copyright symbol
    // (C-in-a-circle), if this symbol is available in the user's
    // locale. Otherwise, do not translate "(C)"; leave it as-is.
    writeln!(stream, "{}", copyright_wrapper.get())?;

    writeln!(stream)?;

    // TRANSLATORS: The %s placeholder is the web address of the GPL license.
    let license_text = unsafe {
        let c_str = CString::new(
            "License GPLv3+: GNU GPL version 3 or later <https://gnu.org/licenses/gpl.html>.\n\
             This is free software: you are free to change and redistribute it.\n\
             There is NO WARRANTY, to the extent permitted by law.",
        )
        .unwrap();
        let result_ptr =
            ::std::ptr::read(&c_str.as_ptr() as *const *const c_char as *const *mut c_char);
        CStr::from_ptr(result_ptr).to_string_lossy().into_owned()
    };
    writeln!(stream, "{}", license_text)?;

    writeln!(stream)?;

    // Print author information based on the number of authors
    match authors.len() {
        0 => {
            // No authors are given. The caller should output authorship
            // info after calling this function.
        }
        1 => {
            // TRANSLATORS: %s denotes an author name.
            let msg = format!("Written by {}.", authors[0]);
            let translated = msg; // Using direct string for simplicity
            writeln!(stream, "{}", translated)?;
        }
        2 => {
            // TRANSLATORS: Each %s denotes an author name.
            let msg = format!("Written by {} and {}.", authors[0], authors[1]);
            let translated = msg; // Using direct string for simplicity
            writeln!(stream, "{}", translated)?;
        }
        3 => {
            // TRANSLATORS: Each %s denotes an author name.
            let msg = format!(
                "Written by {}, {}, and {}.",
                authors[0], authors[1], authors[2]
            );
            let translated = msg; // Using direct string for simplicity
            writeln!(stream, "{}", translated)?;
        }
        4 => {
            // TRANSLATORS: Each %s denotes an author name.
            // You can use line breaks, estimating that each author name occupies
            // ca. 16 screen columns and that a screen line has ca. 80 columns.
            let msg = format!(
                "Written by {}, {}, {},\nand {}.",
                authors[0], authors[1], authors[2], authors[3]
            );
            let translated = msg; // Using direct string for simplicity
            writeln!(stream, "{}", translated)?;
        }
        5 => {
            // TRANSLATORS: Each %s denotes an author name.
            // You can use line breaks, estimating that each author name occupies
            // ca. 16 screen columns and that a screen line has ca. 80 columns.
            let msg = format!(
                "Written by {}, {}, {},\n{}, and {}.",
                authors[0], authors[1], authors[2], authors[3], authors[4]
            );
            let translated = msg; // Using direct string for simplicity
            writeln!(stream, "{}", translated)?;
        }
        6 => {
            // TRANSLATORS: Each %s denotes an author name.
            // You can use line breaks, estimating that each author name occupies
            // ca. 16 screen columns and that a screen line has ca. 80 columns.
            let msg = format!(
                "Written by {}, {}, {},\n{}, {}, and {}.",
                authors[0], authors[1], authors[2], authors[3], authors[4], authors[5]
            );
            let translated = msg; // Using direct string for simplicity
            writeln!(stream, "{}", translated)?;
        }
        7 => {
            // TRANSLATORS: Each %s denotes an author name.
            // You can use line breaks, estimating that each author name occupies
            // ca. 16 screen columns and that a screen line has ca. 80 columns.
            let msg = format!(
                "Written by {}, {}, {},\n{}, {}, {}, and {}.",
                authors[0], authors[1], authors[2], authors[3], authors[4], authors[5], authors[6]
            );
            let translated = msg; // Using direct string for simplicity
            writeln!(stream, "{}", translated)?;
        }
        8 => {
            // TRANSLATORS: Each %s denotes an author name.
            // You can use line breaks, estimating that each author name occupies
            // ca. 16 screen columns and that a screen line has ca. 80 columns.
            let msg = format!(
                "Written by {}, {}, {},\n{}, {}, {}, {},\nand {}.",
                authors[0],
                authors[1],
                authors[2],
                authors[3],
                authors[4],
                authors[5],
                authors[6],
                authors[7]
            );
            let translated = msg; // Using direct string for simplicity
            writeln!(stream, "{}", translated)?;
        }
        9 => {
            // TRANSLATORS: Each %s denotes an author name.
            // You can use line breaks, estimating that each author name occupies
            // ca. 16 screen columns and that a screen line has ca. 80 columns.
            let msg = format!(
                "Written by {}, {}, {},\n{}, {}, {}, {},\n{}, and {}.",
                authors[0],
                authors[1],
                authors[2],
                authors[3],
                authors[4],
                authors[5],
                authors[6],
                authors[7],
                authors[8]
            );
            let translated = msg; // Using direct string for simplicity
            writeln!(stream, "{}", translated)?;
        }
        _ => {
            // 10 or more authors. Use an abbreviation, since the human reader
            // will probably not want to read the entire list anyway.
            // TRANSLATORS: Each %s denotes an author name.
            // You can use line breaks, estimating that each author name occupies
            // ca. 16 screen columns and that a screen line has ca. 80 columns.
            let msg = format!(
                "Written by {}, {}, {},\n{}, {}, {}, {},\n{}, {}, and others.",
                authors[0],
                authors[1],
                authors[2],
                authors[3],
                authors[4],
                authors[5],
                authors[6],
                authors[7],
                authors[8]
            );
            let translated = msg; // Using direct string for simplicity
            writeln!(stream, "{}", translated)?;
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
    n_authors: libc::size_t,
) {
    // Create a wrapper for the version_etc_copyright global variable
    let copyright_wrapper = VersionEtcCopyrightWrapper::new();

    // Convert C strings to Rust strings
    let command_name_opt = if command_name.is_null() {
        None
    } else {
        Some(CStr::from_ptr(command_name).to_string_lossy().into_owned())
    };

    let package_str = CStr::from_ptr(package).to_string_lossy();
    let version_str = CStr::from_ptr(version).to_string_lossy();

    // Convert C array of strings to Rust Vec
    let mut author_vec = Vec::with_capacity(n_authors as usize);
    for i in 0..n_authors {
        let author_ptr = *authors.add(i);
        if !author_ptr.is_null() {
            let author = CStr::from_ptr(author_ptr).to_string_lossy();
            author_vec.push(author);
        }
    }

    // Create a slice of string references for the Rust function
    let author_refs: Vec<&str> = author_vec.iter().map(|s| s.as_ref()).collect();

    // Create a wrapper around the FILE pointer that implements Write
    let file_writer = FileWriter(stream);

    // Call the Rust implementation
    let _ = version_etc_arn_rust(
        file_writer,
        command_name_opt.as_deref(),
        &package_str,
        &version_str,
        &author_refs,
        &copyright_wrapper,
    );
}

// A wrapper around a FILE pointer that implements Write
struct FileWriter(*mut libc::FILE);

impl Write for FileWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let written =
            unsafe { libc::fwrite(buf.as_ptr() as *const libc::c_void, 1, buf.len(), self.0) };

        if written < buf.len() {
            Err(io::Error::last_os_error())
        } else {
            Ok(written)
        }
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

/// Rust implementation of version_etc_va
///
/// This function takes a slice of authors and passes them to version_etc_arn_rust
/// for formatting and output.
pub fn version_etc_va_rust<W: Write>(
    stream: W,
    command_name: Option<&str>,
    package: &str,
    version: &str,
    authors: &[&str],
    copyright_wrapper: &VersionEtcCopyrightWrapper,
) -> io::Result<()> {
    version_etc_arn_rust(
        stream,
        command_name,
        package,
        version,
        authors,
        copyright_wrapper,
    )
}
// We need to use a different approach since we can't handle va_list in stable Rust
// and version_etc_arn is already defined

// This is a type alias to represent the va_list type from C
// It's opaque to Rust, we just pass it through
type CVaList = *mut libc::c_void;

#[no_mangle]

/// Rust implementation of version_etc that takes a list of authors as a slice.
pub fn version_etc_rust<W: Write>(
    stream: &mut W,
    command_name: Option<&str>,
    package: &str,
    version: &str,
    authors: &[&str],
) -> io::Result<()> {
    // Call the existing version_etc_va_rust function
    // Assuming version_etc_va_rust and VersionEtcCopyrightWrapper are already defined elsewhere
    let copyright_wrapper = crate::VersionEtcCopyrightWrapper::new();
    crate::version_etc_va_rust(
        stream,
        command_name,
        package,
        version,
        authors,
        &copyright_wrapper,
    )
}
#[no_mangle]

#[derive(Debug, Clone)]
pub struct stat_rust {
    pub st_dev: u64,        // Device
    pub st_ino: u64,        // File serial number
    pub st_mode: u32,       // File mode
    pub st_nlink: u64,      // Link count
    pub st_uid: u32,        // User ID of the file's owner
    pub st_gid: u32,        // Group ID of the file's group
    pub st_rdev: u64,       // Device number, if device
    pub st_size: i64,       // Size of file, in bytes
    pub st_blksize: i64,    // Optimal block size for I/O
    pub st_blocks: i64,     // 512-byte blocks
    pub st_atime: Duration, // Time of last access
    pub st_mtime: Duration, // Time of last modification
    pub st_ctime: Duration, // Time of last status change
}

fn logical_getcwd_rust() -> Option<String> {
    // Get PWD from environment
    let wd = match env::var("PWD") {
        Ok(pwd) => pwd,
        Err(_) => return None,
    };

    // Textual validation first
    if !wd.starts_with('/') {
        return None;
    }

    // Check for problematic path components like "/." or "/.."
    let mut i = 0;
    while let Some(pos) = wd[i..].find("/.") {
        let real_pos = i + pos;
        let after_dot = real_pos + 2;

        if after_dot >= wd.len()
            || wd.as_bytes()[after_dot] == b'/'
            || (after_dot + 1 <= wd.len()
                && wd.as_bytes()[after_dot] == b'.'
                && (after_dot + 1 == wd.len() || wd.as_bytes()[after_dot + 1] == b'/'))
        {
            return None;
        }

        i = real_pos + 1;
    }

    // System call validation
    match (fs::metadata(&wd), fs::metadata(".")) {
        (Ok(st1), Ok(st2)) => {
            if st1.dev() == st2.dev() && st1.ino() == st2.ino() {
                Some(wd)
            } else {
                None
            }
        }
        _ => None,
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


#[repr(C)]

// Define an idiomatic Rust version of quoting_options
#[derive(Clone)]
pub struct QuotingOptions {
    // Since we don't know the actual structure, I'll create a placeholder
    // that can be expanded based on the actual needs
    pub unused: i32,
}

// Thread-safe wrapper for the global variable
pub struct QuoteQuotingOptionsWrapper {
    val: QuotingOptions,
}

impl QuoteQuotingOptionsWrapper {
    
    
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
    val: Vec<GetoptOption>,
}

impl LongoptsWrapper {
    pub fn new() -> Self {
        // Read the current value of the global variable
        let current_val = unsafe {
            // Convert the static array to a Vec
            let slice = &longopts[..];
            slice
                .iter()
                .map(|opt| GetoptOption {
                    name: if opt.name.is_null() {
                        None
                    } else {
                        Some(
                            unsafe { CStr::from_ptr(opt.name) }
                                .to_string_lossy()
                                .into_owned(),
                        )
                    },
                    has_arg: opt.has_arg != 0,
                    flag: if opt.flag.is_null() {
                        None
                    } else {
                        Some(unsafe { Box::new(*opt.flag != 0) })
                    },
                    val: char::from_u32(opt.val as u32).unwrap_or('\0'),
                })
                .collect()
        };

        Self { val: current_val }
    }

    
    }

// Idiomatic Rust representation of the option struct
pub struct GetoptOption {
    pub name: Option<String>,
    pub has_arg: bool,
    pub flag: Option<Box<bool>>,
    pub val: char,
}

fn main_0_rust(
    args: Vec<String>,
    program_name_wrapper: &mut ProgramNameWrapper,
    version_wrapper: &VersionWrapper,
    longopts_wrapper: &LongoptsWrapper,
) -> i32 {
    // Set program name
    set_program_name_rust(&args[0], program_name_wrapper);

    // Set locale and text domain
    let c_locale = CString::new("").unwrap();
    let c_coreutils = CString::new("coreutils").unwrap();
    let c_locale_path = CString::new("/usr/local/share/locale").unwrap();

    unsafe {
        setlocale(6, c_locale.as_ptr());
        bindtextdomain(c_coreutils.as_ptr(), c_locale_path.as_ptr());
        textdomain(c_coreutils.as_ptr());

        // Register close_stdout at exit
        atexit(Some(close_stdout));
    }

    // Check if POSIXLY_CORRECT is set
    let mut logical = match env::var("POSIXLY_CORRECT") {
        Ok(_) => true,
        Err(_) => false,
    };

    // Parse command line arguments manually
    let mut i = 1;
    let mut non_option_args = false;

    while i < args.len() {
        match args[i].as_str() {
            "-L" => logical = true,
            "-P" => logical = false,
            "--logical" => logical = true,
            "--physical" => logical = false,
            "--help" => {
                unsafe { usage(0) };
                return 0;
            }
            "--version" => {
                let mut std_stdout = std::io::stdout();
                let _ = version_etc_rust(
                    &mut std_stdout,
                    Some("pwd"),
                    "GNU coreutils",
                    &version_wrapper.get(),
                    &[&proper_name_lite_rust("Jim Meyering", "Jim Meyering")],
                );
                process::exit(0);
            }
            arg if arg.starts_with('-') => {
                // Invalid option
                eprintln!("Try 'pwd --help' for more information.");
                return 1;
            }
            _ => {
                // Non-option argument
                non_option_args = true;
            }
        }
        i += 1;
    }

    if non_option_args {
        eprintln!("ignoring non-option arguments");
    }

    // Try to get the current working directory
    if logical {
        if let Some(wd) = logical_getcwd_rust() {
            println!("{}", wd);
            return 0;
        }
    }

    // If logical_getcwd failed or wasn't requested, try xgetcwd
    if let Some(wd) = xgetcwd_rust() {
        println!("{}", wd);
        return 0;
    } else {
        // As a fallback, use std::env::current_dir
        match env::current_dir() {
            Ok(path) => {
                if let Some(path_str) = path.to_str() {
                    println!("{}", path_str);
                } else {
                    eprintln!("Error: current directory path contains invalid UTF-8");
                    return 1;
                }
            }
            Err(e) => {
                eprintln!("Error getting current directory: {}", e);
                return 1;
            }
        }
    }

    0
}
#[no_mangle]
pub unsafe extern "C" fn main_0(argc: libc::c_int, argv: *mut *mut libc::c_char) -> libc::c_int {
    let mut args = Vec::new();

    // Convert C arguments to Rust strings
    for i in 0..argc {
        let arg_ptr = *argv.offset(i as isize);
        let c_str = CStr::from_ptr(arg_ptr);
        let arg_str = c_str.to_string_lossy().into_owned();
        args.push(arg_str);
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
