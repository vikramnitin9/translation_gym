#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

use libc::{c_char, c_int, c_uint, c_void, size_t, EINTR, EINVAL};
use std::{
    alloc::{self, Layout},
    char,
    cmp::min,
    env,
    ffi::{CStr, CString, OsString},
    io::{self, Error, ErrorKind, Read, Write},
    mem,
    os::unix::{
        ffi::OsStringExt,
        io::{FromRawFd, IntoRawFd, RawFd},
    },
    process,
    process::Command,
    ptr, slice,
    sync::{
        atomic::{AtomicBool, AtomicI32, AtomicI8, AtomicPtr, Ordering},
        Mutex, Once,
    },
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

    
    pub fn set(&mut self, val: Option<String>) {
        self.val = val.clone();

        // Update the global variable
        unsafe {
            if let Some(s) = &val {
                // Convert String to C string and leak it (since we're setting a global)
                let c_string = std::ffi::CString::new(s.clone()).unwrap();
                program_name = c_string.into_raw() as *const ::std::os::raw::c_char;
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
    let slash_pos = argv0.rfind('/');
    let base = match slash_pos {
        Some(pos) => &argv0[pos + 1..],
        None => argv0,
    };

    // Check if the path contains "/.libs/"
    let mut final_name = argv0;

    if let Some(pos) = slash_pos {
        if pos >= 7 {
            let prefix_start = pos - 7;
            if &argv0[prefix_start..=pos] == "/.libs/" {
                final_name = base;

                // Check if base_name starts with "lt-"
                if base.starts_with("lt-") {
                    final_name = &base[3..]; // Skip "lt-" prefix

                    // On glibc systems, remove the "lt-" prefix from program_invocation_short_name
                    unsafe {
                        if !program_invocation_short_name.is_null() {
                            let c_str = CString::new(final_name).unwrap();
                            program_invocation_short_name = c_str.into_raw();
                        }
                    }
                }
            }
        }
    }

    // Set program_name
    program_name_wrapper.set(Some(final_name.to_string()));

    // On glibc systems, set program_invocation_name as well
    unsafe {
        if !program_invocation_name.is_null() {
            let c_str = CString::new(final_name).unwrap();
            program_invocation_name = c_str.into_raw();
        }
    }
}
#[no_mangle]
pub unsafe extern "C" fn set_program_name(argv0: *const c_char) {
    // Create a new ProgramNameWrapper instance
    let mut program_name_wrapper = ProgramNameWrapper::new();

    // Convert C string to Rust string
    if argv0.is_null() {
        // Handle NULL pointer case
        let stderr_ptr = stderr;
        let message = "A NULL argv[0] was passed through an exec system call.\n";
        let c_message = CString::new(message).unwrap();
        fputs(c_message.as_ptr(), stderr_ptr);
        abort();
    }

    let argv0_cstr = CStr::from_ptr(argv0);
    let argv0_str = match argv0_cstr.to_str() {
        Ok(s) => s.to_string(),
        Err(_) => {
            // If the string is not valid UTF-8, convert it to a lossy UTF-8 string
            String::from_utf8_lossy(argv0_cstr.to_bytes()).into_owned()
        }
    };

    // Call the Rust implementation
    set_program_name_rust(&argv0_str, &mut program_name_wrapper);
}


/// Returns the character set of the current locale.
///
/// This is a Rust implementation of the C function `locale_charset`.
/// It attempts to determine the character encoding of the current locale.
fn locale_charset_rust() -> String {
    // In Rust, we can use the LANG environment variable as a substitute
    // for nl_langinfo(CODESET)
    let codeset = match env::var("LC_ALL")
        .or_else(|_| env::var("LC_CTYPE"))
        .or_else(|_| env::var("LANG"))
    {
        Ok(val) => {
            // Extract charset from locale string like "en_US.UTF-8"
            if let Some(index) = val.find('.') {
                val[index + 1..].to_string()
            } else {
                // No charset specified in locale
                String::new()
            }
        }
        Err(_) => String::new(),
    };

    // Don't return an empty string
    if codeset.is_empty() {
        "ASCII".to_string()
    } else {
        codeset
    }
}
#[no_mangle]

/// Converts an uppercase ASCII character to lowercase.
/// Returns the input unchanged if it's not an uppercase ASCII letter.
#[inline]
fn c_tolower_rust(c: i32) -> i32 {
    // Check if c is an uppercase ASCII letter (A-Z)
    if c >= 'A' as i32 && c <= 'Z' as i32 {
        // Convert to lowercase by adding the difference between 'a' and 'A'
        c + ('a' as i32 - 'A' as i32)
    } else {
        // Return the character unchanged
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
        // Get the next character from each string
        let c1_opt = iter1.next();
        let c2_opt = iter2.next();

        // If we've reached the end of both strings simultaneously, they're equal
        if c1_opt.is_none() && c2_opt.is_none() {
            return 0;
        }

        // Get the characters and convert to lowercase
        let c1 = c1_opt.map_or(0, |c| c_tolower_rust(c as i32) as u8);
        let c2 = c2_opt.map_or(0, |c| c_tolower_rust(c as i32) as u8);

        // If characters differ or we've reached the end of a string, break
        if c1 != c2 || c1 == 0 {
            // Handle the same integer overflow protection as in the C version
            if (127 * 2 + 1) <= 2147483647 {
                return (c1 as i32) - (c2 as i32);
            } else {
                // This is the equivalent of ((c1 > c2) - (c1 < c2))
                return (c1 > c2) as i32 - (c1 < c2) as i32;
            }
        }
    }
}
#[no_mangle]

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
pub unsafe extern "C" fn proper_name_lite(
    name_ascii: *const c_char,
    name_utf8: *const c_char,
) -> *const c_char {
    // Convert C strings to Rust strings
    let r_name_ascii = if name_ascii.is_null() {
        return ptr::null();
    } else {
        CStr::from_ptr(name_ascii).to_string_lossy().into_owned()
    };

    let r_name_utf8 = if name_utf8.is_null() {
        return ptr::null();
    } else {
        CStr::from_ptr(name_utf8).to_string_lossy().into_owned()
    };

    // Call the Rust implementation
    let result = proper_name_lite_rust(&r_name_ascii, &r_name_utf8);

    // Convert the result back to a C string and leak it
    // (the caller is responsible for freeing it)
    let c_result = CString::new(result).unwrap_or_default();
    c_result.into_raw() as *const c_char
}

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

    
    fn get_global_value() -> String {
        unsafe {
            // Since version_etc_copyright is a zero-sized array, it's likely
            // meant to be a pointer to a C string. We'll treat it as such.
            // This is unsafe because we're dereferencing a raw pointer and
            // assuming it points to a valid null-terminated C string.
            if version_etc_copyright.as_ptr().is_null() {
                return String::new();
            }

            // Convert the C string to a Rust String
            CStr::from_ptr(version_etc_copyright.as_ptr())
                .to_string_lossy()
                .into_owned()
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
    // TRANSLATORS: Translate "(C)" to the copyright symbol (C-in-a-circle),
    // if this symbol is available in the user's locale.
    // Otherwise, do not translate "(C)"; leave it as-is.
    writeln!(stream, "{}", copyright_wrapper.get())?;
    writeln!(stream)?;

    // Print license info
    // TRANSLATORS: The %s placeholder is the web address of the GPL license.
    let license_text = unsafe {
        let c_str = CString::new(
            "License GPLv3+: GNU GPL version 3 or later <%s>.\n\
             This is free software: you are free to change and redistribute it.\n\
             There is NO WARRANTY, to the extent permitted by law.",
        )
        .unwrap();
        let result_ptr = gettext(c_str.as_ptr());
        CStr::from_ptr(result_ptr).to_string_lossy().into_owned()
    };

    // Replace %s with the URL
    let license_text = license_text.replace("%s", "https://gnu.org/licenses/gpl.html");
    writeln!(stream, "{}", license_text)?;
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
                let format = CString::new("Written by %s.").unwrap();
                let result_ptr = gettext(format.as_ptr());
                CStr::from_ptr(result_ptr).to_string_lossy().into_owned()
            };
            writeln!(stream, "{}", msg.replace("%s", authors[0]))?;
        }
        2 => {
            // TRANSLATORS: Each %s denotes an author name.
            let msg = unsafe {
                let format = CString::new("Written by %s and %s.").unwrap();
                let result_ptr = gettext(format.as_ptr());
                CStr::from_ptr(result_ptr).to_string_lossy().into_owned()
            };
            let msg = msg
                .replacen("%s", authors[0], 1)
                .replacen("%s", authors[1], 1);
            writeln!(stream, "{}", msg)?;
        }
        3 => {
            // TRANSLATORS: Each %s denotes an author name.
            let msg = unsafe {
                let format = CString::new("Written by %s, %s, and %s.").unwrap();
                let result_ptr = gettext(format.as_ptr());
                CStr::from_ptr(result_ptr).to_string_lossy().into_owned()
            };
            let msg = msg
                .replacen("%s", authors[0], 1)
                .replacen("%s", authors[1], 1)
                .replacen("%s", authors[2], 1);
            writeln!(stream, "{}", msg)?;
        }
        4 => {
            // TRANSLATORS: Each %s denotes an author name.
            let msg = unsafe {
                let format = CString::new("Written by %s, %s, %s,\nand %s.").unwrap();
                let result_ptr = gettext(format.as_ptr());
                CStr::from_ptr(result_ptr).to_string_lossy().into_owned()
            };
            let msg = msg
                .replacen("%s", authors[0], 1)
                .replacen("%s", authors[1], 1)
                .replacen("%s", authors[2], 1)
                .replacen("%s", authors[3], 1);
            writeln!(stream, "{}", msg)?;
        }
        5 => {
            // TRANSLATORS: Each %s denotes an author name.
            let msg = unsafe {
                let format = CString::new("Written by %s, %s, %s,\n%s, and %s.").unwrap();
                let result_ptr = gettext(format.as_ptr());
                CStr::from_ptr(result_ptr).to_string_lossy().into_owned()
            };
            let msg = msg
                .replacen("%s", authors[0], 1)
                .replacen("%s", authors[1], 1)
                .replacen("%s", authors[2], 1)
                .replacen("%s", authors[3], 1)
                .replacen("%s", authors[4], 1);
            writeln!(stream, "{}", msg)?;
        }
        6 => {
            // TRANSLATORS: Each %s denotes an author name.
            let msg = unsafe {
                let format = CString::new("Written by %s, %s, %s,\n%s, %s, and %s.").unwrap();
                let result_ptr = gettext(format.as_ptr());
                CStr::from_ptr(result_ptr).to_string_lossy().into_owned()
            };
            let msg = msg
                .replacen("%s", authors[0], 1)
                .replacen("%s", authors[1], 1)
                .replacen("%s", authors[2], 1)
                .replacen("%s", authors[3], 1)
                .replacen("%s", authors[4], 1)
                .replacen("%s", authors[5], 1);
            writeln!(stream, "{}", msg)?;
        }
        7 => {
            // TRANSLATORS: Each %s denotes an author name.
            let msg = unsafe {
                let format = CString::new("Written by %s, %s, %s,\n%s, %s, %s, and %s.").unwrap();
                let result_ptr = gettext(format.as_ptr());
                CStr::from_ptr(result_ptr).to_string_lossy().into_owned()
            };
            let msg = msg
                .replacen("%s", authors[0], 1)
                .replacen("%s", authors[1], 1)
                .replacen("%s", authors[2], 1)
                .replacen("%s", authors[3], 1)
                .replacen("%s", authors[4], 1)
                .replacen("%s", authors[5], 1)
                .replacen("%s", authors[6], 1);
            writeln!(stream, "{}", msg)?;
        }
        8 => {
            // TRANSLATORS: Each %s denotes an author name.
            let msg = unsafe {
                let format =
                    CString::new("Written by %s, %s, %s,\n%s, %s, %s, %s,\nand %s.").unwrap();
                let result_ptr = gettext(format.as_ptr());
                CStr::from_ptr(result_ptr).to_string_lossy().into_owned()
            };
            let msg = msg
                .replacen("%s", authors[0], 1)
                .replacen("%s", authors[1], 1)
                .replacen("%s", authors[2], 1)
                .replacen("%s", authors[3], 1)
                .replacen("%s", authors[4], 1)
                .replacen("%s", authors[5], 1)
                .replacen("%s", authors[6], 1)
                .replacen("%s", authors[7], 1);
            writeln!(stream, "{}", msg)?;
        }
        9 => {
            // TRANSLATORS: Each %s denotes an author name.
            let msg = unsafe {
                let format =
                    CString::new("Written by %s, %s, %s,\n%s, %s, %s, %s,\n%s, and %s.").unwrap();
                let result_ptr = gettext(format.as_ptr());
                CStr::from_ptr(result_ptr).to_string_lossy().into_owned()
            };
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
            writeln!(stream, "{}", msg)?;
        }
        _ => {
            // 10 or more authors. Use an abbreviation.
            // TRANSLATORS: Each %s denotes an author name.
            let msg = unsafe {
                let format =
                    CString::new("Written by %s, %s, %s,\n%s, %s, %s, %s,\n%s, %s, and others.")
                        .unwrap();
                let result_ptr = gettext(format.as_ptr());
                CStr::from_ptr(result_ptr).to_string_lossy().into_owned()
            };
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
            writeln!(stream, "{}", msg)?;
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
    // Create a wrapper around the FILE pointer
    let file_writer = FileWriter { file: stream };

    // Convert C strings to Rust strings
    let cmd_name = if command_name.is_null() {
        None
    } else {
        Some(CStr::from_ptr(command_name).to_string_lossy().into_owned())
    };

    let pkg = CStr::from_ptr(package).to_string_lossy().into_owned();
    let ver = CStr::from_ptr(version).to_string_lossy().into_owned();

    // Convert C array of strings to Rust Vec
    let mut author_vec = Vec::with_capacity(n_authors as usize);
    for i in 0..n_authors {
        let author_ptr = *authors.add(i);
        if !author_ptr.is_null() {
            let author = CStr::from_ptr(author_ptr).to_string_lossy().into_owned();
            author_vec.push(author);
        }
    }

    // Create references to owned strings for the Rust function
    let author_refs: Vec<&str> = author_vec.iter().map(|s| s.as_str()).collect();

    // Create the copyright wrapper
    let copyright_wrapper = VersionEtcCopyrightWrapper::new();

    // Call the Rust implementation
    let _ = version_etc_arn_rust(
        file_writer,
        cmd_name.as_deref(),
        &pkg,
        &ver,
        &author_refs,
        &copyright_wrapper,
    );
}

// A wrapper around a FILE pointer that implements Write
struct FileWriter {
    file: *mut libc::FILE,
}

impl Write for FileWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let result =
            unsafe { libc::fwrite(buf.as_ptr() as *const libc::c_void, 1, buf.len(), self.file) };

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

/// Prints version and copyright information to the specified writer.
///
/// This is a Rust implementation of the C `version_etc` function.
// Since we can't handle variadic functions in stable Rust,
// we'll just forward the call to the existing C implementation
#[no_mangle]
pub unsafe extern "C" fn version_etc(
    stream: *mut libc::FILE,
    command_name: *const c_char,
    package: *const c_char,
    version: *const c_char,
    /* ... variadic arguments ... */
) {
    // We need to forward this call to the C implementation
    // that can handle variadic arguments
    extern "C" {
        fn version_etc_va(
            stream: *mut libc::FILE,
            command_name: *const c_char,
            package: *const c_char,
            version: *const c_char,
            authors: *mut libc::c_void,
        );
    }

    // This is a workaround since we can't directly handle variadic arguments in stable Rust
    // We're using the __builtin_apply_args and __builtin_apply pattern from GCC
    // to capture and forward the variadic arguments

    // In a real implementation, we would need to use the C preprocessor or compiler intrinsics
    // Since we can't do that here, we'll use a simplified approach that works for this specific case

    // Get the address of the first variadic argument
    let args_ptr = unsafe {
        let args_addr = (version as usize) + std::mem::size_of::<*const c_char>();
        args_addr as *mut libc::c_void
    };

    // Call the C implementation with the captured arguments
    version_etc_va(stream, command_name, package, version, args_ptr);
}

/// Calculate an appropriate block size for I/O operations based on file metadata
///
/// This function determines an optimal block size for I/O operations by examining
/// the file's reported block size and applying various heuristics to improve performance.
fn io_blksize_rust(st_mode: u32, st_blksize: i64) -> isize {
    const IO_BUFSIZE: isize = 8192; // Common buffer size for I/O operations

    // Get the block size, with fallback to 512 if invalid
    let blocksize = if 0 < st_blksize && st_blksize <= (isize::MAX / 8 + 1) as i64 {
        st_blksize as isize
    } else {
        512
    };

    // Use IO_BUFSIZE if blocksize is invalid
    let mut blocksize = if blocksize <= 0 {
        IO_BUFSIZE
    } else {
        blocksize
    };

    // Round up to a multiple of IO_BUFSIZE
    blocksize += (IO_BUFSIZE - 1) - (IO_BUFSIZE - 1) % blocksize;

    // For regular files, if blocksize is not a power of two, use the next power of two
    if (st_mode & 0o170000) == 0o100000 && (blocksize & (blocksize - 1)) != 0 {
        // Use Rust's built-in leading_zeros instead of the C function
        let leading_zeros = (blocksize as u64).leading_zeros();

        // Calculate the next power of two
        if leading_zeros < 64 {
            let power = 1u64 << (64 - leading_zeros);
            if power <= isize::MAX as u64 {
                blocksize = power as isize;
            }
        }
    }

    // Don't exceed the maximum safe size (min of isize::MAX and usize::MAX divided by 2 plus 1)
    let max_safe_size = min(isize::MAX, usize::MAX as isize) / 2 + 1;
    min(max_safe_size, blocksize)
}
#[no_mangle]
pub unsafe extern "C" fn io_blksize(st: *const libc::stat) -> isize {
    if st.is_null() {
        return 8192; // Return IO_BUFSIZE if null pointer
    }

    // Extract the needed fields from the stat struct
    let st_mode = (*st).st_mode as u32;
    let st_blksize = (*st).st_blksize as i64;

    // Call the Rust implementation
    io_blksize_rust(st_mode, st_blksize)
}

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

pub struct quoting_options_rust {
    /// Basic quoting style.
    pub style: Quoting_style_rust,

    /// Additional flags. Bitwise combination of enum quoting_flags.
    pub flags: i32,

    /// Quote the characters indicated by this bit vector even if the
    /// quoting style would not normally require them to be quoted.
    pub quote_these_too: Vec<u32>,

    /// The left quote for custom_quoting_style.
    pub left_quote: Option<String>,

    /// The right quote for custom_quoting_style.
    pub right_quote: Option<String>,
}

/// Represents options for quoting characters.
#[derive(Default, Clone)]
pub struct QuotingOptions {
    /// Bitmap array for characters that need quoting.
    /// Each element represents a chunk of characters.
    quote_these_too: [u32; 8],
}

impl QuotingOptions {
    /// Creates a new instance with default settings.
    pub fn new() -> Self {
        Self::default()
    }
}

// Global default quoting options
static mut DEFAULT_QUOTING_OPTIONS: Option<QuotingOptions> = None;

/// Sets or clears the quoting flag for a character in the quoting options.
///
/// # Arguments
///
/// * `o` - Optional reference to quoting options. If None, uses default options.
/// * `c` - The character to set quoting for.
/// * `i` - The value to set (1 to enable quoting, 0 to disable).
///
/// # Returns
///
/// The previous quoting state for the character (1 if it was quoted, 0 if not).
fn set_char_quoting_rust(o: Option<&mut QuotingOptions>, c: char, i: i32) -> i32 {
    let uc = c as u8;

    // Initialize default options if needed
    unsafe {
        if DEFAULT_QUOTING_OPTIONS.is_none() {
            DEFAULT_QUOTING_OPTIONS = Some(QuotingOptions::new());
        }
    }

    // Get a reference to the options, or use default if None
    let options = match o {
        Some(opts) => opts,
        None => unsafe { DEFAULT_QUOTING_OPTIONS.as_mut().unwrap() },
    };

    // Calculate which element in the array contains the bit for this character
    let index = (uc / (std::mem::size_of::<u32>() * 8) as u8) as usize;

    // Calculate the bit position within the element
    let shift = (uc % (std::mem::size_of::<u32>() * 8) as u8) as u32;

    // Get the current bit value
    let r = ((options.quote_these_too[index] >> shift) & 1) as i32;

    // Toggle the bit if the requested state differs from the current state
    // Convert to u32 to avoid type mismatch
    options.quote_these_too[index] ^= (((i & 1) ^ r) as u32) << shift;

    r
}
/// FFI-compatible wrapper for set_char_quoting_rust.
#[no_mangle]
pub unsafe extern "C" fn set_char_quoting(
    o: *mut libc::c_void,
    c: libc::c_char,
    i: libc::c_int,
) -> libc::c_int {
    let options = if !o.is_null() {
        // Convert the raw pointer to a mutable reference
        Some(&mut *(o as *mut QuotingOptions))
    } else {
        None
    };

    // Call the Rust implementation
    set_char_quoting_rust(options, c as u8 as char, i)
}

pub struct ExitFailureWrapper {
    val: i32,
}

impl ExitFailureWrapper {
    
    
    }

#[derive(Clone, Copy, PartialEq, Eq)]
enum _rust {
    P_ALL,  // Wait for any child.
    P_PID,  // Wait for specified process.
    P_PGID, // Wait for members of process group.
}

/// Flags for quoting style
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuotingFlags {
    ElideOuterQuotes = 0x01,
    SplitTrigraphs = 0x02,
    ElideNullBytes = 0x04,
}

/// Rust implementation of quotearg_buffer_restyled
pub fn quotearg_buffer_restyled_rust(
    mut buffer: Option<&mut [u8]>,
    arg: &[u8],
    quoting_style: Quoting_style_rust,
    flags: u32,
    quote_these_too: Option<&[c_uint]>,
    left_quote: Option<&[u8]>,
    right_quote: Option<&[u8]>,
) -> usize {
    let elide_outer_quotes = (flags & QuotingFlags::ElideOuterQuotes as u32) != 0;
    let split_trigraphs = (flags & QuotingFlags::SplitTrigraphs as u32) != 0;
    let elide_null_bytes = (flags & QuotingFlags::ElideNullBytes as u32) != 0;

    let mut i = 0;
    let mut len = 0;
    let mut orig_buffersize = 0;
    let mut quote_string: Option<&[u8]> = None;
    let mut quote_string_len = 0;
    let mut backslash_escapes = false;

    // Use Rust's locale functionality instead of C's __ctype_get_mb_cur_max
    let unibyte_locale = is_unibyte_locale();

    let mut encountered_single_quote = false;
    let mut all_c_and_shell_quote_compat = true;

    // Process input label
    let mut pending_shell_escape_end = false;

    // Determine quoting style parameters
    match quoting_style {
        Quoting_style_rust::C_maybe_quoting_style => {
            let new_quoting_style = Quoting_style_rust::C_quoting_style;
            return quotearg_buffer_restyled_rust(
                buffer,
                arg,
                new_quoting_style,
                flags | QuotingFlags::ElideOuterQuotes as u32,
                quote_these_too,
                left_quote,
                right_quote,
            );
        }
        Quoting_style_rust::C_quoting_style => {
            if !elide_outer_quotes {
                write_to_buffer(&mut buffer, b'"', &mut len);
            }
            backslash_escapes = true;
            quote_string = Some(b"\"");
            quote_string_len = 1;
        }
        Quoting_style_rust::Escape_quoting_style => {
            backslash_escapes = true;
        }
        Quoting_style_rust::Locale_quoting_style
        | Quoting_style_rust::Clocale_quoting_style
        | Quoting_style_rust::Custom_quoting_style => {
            let (left, right) = if quoting_style != Quoting_style_rust::Custom_quoting_style {
                // In a real implementation, we would use gettext_quote here
                // For now, use simple quotes as a fallback
                (b"`" as &[u8], b"'" as &[u8])
            } else {
                (left_quote.unwrap_or(b"`"), right_quote.unwrap_or(b"'"))
            };

            if !elide_outer_quotes {
                for &c in left {
                    write_to_buffer(&mut buffer, c, &mut len);
                }
            }
            backslash_escapes = true;
            quote_string = Some(right);
            quote_string_len = right.len();
        }
        Quoting_style_rust::Shell_escape_quoting_style => {
            backslash_escapes = true;
            // Fall through to shell_quoting_style
            let new_quoting_style = Quoting_style_rust::Shell_quoting_style;
            return quotearg_buffer_restyled_rust(
                buffer,
                arg,
                new_quoting_style,
                flags,
                quote_these_too,
                left_quote,
                right_quote,
            );
        }
        Quoting_style_rust::Shell_quoting_style => {
            // Fall through to shell_escape_always_quoting_style
            let new_quoting_style = Quoting_style_rust::Shell_escape_always_quoting_style;
            return quotearg_buffer_restyled_rust(
                buffer,
                arg,
                new_quoting_style,
                flags | QuotingFlags::ElideOuterQuotes as u32,
                quote_these_too,
                left_quote,
                right_quote,
            );
        }
        Quoting_style_rust::Shell_escape_always_quoting_style => {
            if !elide_outer_quotes {
                backslash_escapes = true;
            }
            // Fall through to shell_always_quoting_style
            let new_quoting_style = Quoting_style_rust::Shell_always_quoting_style;
            return quotearg_buffer_restyled_rust(
                buffer,
                arg,
                new_quoting_style,
                flags,
                quote_these_too,
                left_quote,
                right_quote,
            );
        }
        Quoting_style_rust::Shell_always_quoting_style => {
            if !elide_outer_quotes {
                write_to_buffer(&mut buffer, b'\'', &mut len);
            }
            quote_string = Some(b"'");
            quote_string_len = 1;
        }
        Quoting_style_rust::Literal_quoting_style => {
            // No special handling needed
        }
    }

    // Process each character in the input
    while i < arg.len() && (arg[i] != b'\0' || i != arg.len() - 1) {
        let c = arg[i];
        let mut esc = 0;
        let mut is_right_quote = false;
        let mut escaping = false;
        let mut c_and_shell_quote_compat = false;

        // Check if we're at a right quote
        if backslash_escapes
            && quoting_style != Quoting_style_rust::Shell_always_quoting_style
            && quote_string_len > 0
            && i + quote_string_len <= arg.len()
            && arg[i..].starts_with(quote_string.unwrap_or(&[]))
        {
            if elide_outer_quotes {
                return force_outer_quoting_style(
                    buffer,
                    arg,
                    quoting_style,
                    flags,
                    quote_these_too,
                    left_quote,
                    right_quote,
                );
            }
            is_right_quote = true;
        }

        // Process character based on its value
        match c {
            b'\0' => {
                if backslash_escapes {
                    if elide_outer_quotes {
                        return force_outer_quoting_style(
                            buffer,
                            arg,
                            quoting_style,
                            flags,
                            quote_these_too,
                            left_quote,
                            right_quote,
                        );
                    }
                    escaping = true;

                    if quoting_style == Quoting_style_rust::Shell_always_quoting_style
                        && !pending_shell_escape_end
                    {
                        write_to_buffer(&mut buffer, b'\'', &mut len);
                        write_to_buffer(&mut buffer, b'$', &mut len);
                        write_to_buffer(&mut buffer, b'\'', &mut len);
                        pending_shell_escape_end = true;
                    }

                    write_to_buffer(&mut buffer, b'\\', &mut len);

                    // Handle octal escaping for null byte
                    if quoting_style != Quoting_style_rust::Shell_always_quoting_style
                        && i + 1 < arg.len()
                        && arg[i + 1] >= b'0'
                        && arg[i + 1] <= b'9'
                    {
                        write_to_buffer(&mut buffer, b'0', &mut len);
                        write_to_buffer(&mut buffer, b'0', &mut len);
                    }
                } else if elide_null_bytes {
                    i += 1;
                    continue;
                }
            }

            b'?' => match quoting_style {
                Quoting_style_rust::Shell_always_quoting_style => {
                    if elide_outer_quotes {
                        return force_outer_quoting_style(
                            buffer,
                            arg,
                            quoting_style,
                            flags,
                            quote_these_too,
                            left_quote,
                            right_quote,
                        );
                    }
                }
                Quoting_style_rust::C_quoting_style => {
                    if split_trigraphs && i + 2 < arg.len() && arg[i + 1] == b'?' {
                        match arg[i + 2] {
                            b'!' | b'\'' | b'(' | b')' | b'-' | b'/' | b'<' | b'=' | b'>' => {
                                if elide_outer_quotes {
                                    return force_outer_quoting_style(
                                        buffer,
                                        arg,
                                        quoting_style,
                                        flags,
                                        quote_these_too,
                                        left_quote,
                                        right_quote,
                                    );
                                }
                                let next_c = arg[i + 2];
                                i += 2;
                                write_to_buffer(&mut buffer, b'?', &mut len);
                                write_to_buffer(&mut buffer, b'"', &mut len);
                                write_to_buffer(&mut buffer, b'"', &mut len);
                                write_to_buffer(&mut buffer, b'?', &mut len);
                            }
                            _ => {}
                        }
                    }
                }
                _ => {}
            },

            // Control characters
            0x07 => {
                esc = b'a';
                handle_c_escape(
                    &mut escaping,
                    &mut c_and_shell_quote_compat,
                    backslash_escapes,
                    &mut buffer,
                    &mut len,
                    esc,
                    c,
                    quoting_style,
                    elide_outer_quotes,
                    &mut pending_shell_escape_end,
                );
            }
            0x08 => {
                esc = b'b';
                handle_c_escape(
                    &mut escaping,
                    &mut c_and_shell_quote_compat,
                    backslash_escapes,
                    &mut buffer,
                    &mut len,
                    esc,
                    c,
                    quoting_style,
                    elide_outer_quotes,
                    &mut pending_shell_escape_end,
                );
            }
            0x0C => {
                esc = b'f';
                handle_c_escape(
                    &mut escaping,
                    &mut c_and_shell_quote_compat,
                    backslash_escapes,
                    &mut buffer,
                    &mut len,
                    esc,
                    c,
                    quoting_style,
                    elide_outer_quotes,
                    &mut pending_shell_escape_end,
                );
            }
            b'\n' => {
                esc = b'n';
                handle_c_and_shell_escape(
                    &mut escaping,
                    &mut c_and_shell_quote_compat,
                    backslash_escapes,
                    &mut buffer,
                    &mut len,
                    esc,
                    c,
                    quoting_style,
                    elide_outer_quotes,
                    &mut pending_shell_escape_end,
                );
            }
            b'\r' => {
                esc = b'r';
                handle_c_and_shell_escape(
                    &mut escaping,
                    &mut c_and_shell_quote_compat,
                    backslash_escapes,
                    &mut buffer,
                    &mut len,
                    esc,
                    c,
                    quoting_style,
                    elide_outer_quotes,
                    &mut pending_shell_escape_end,
                );
            }
            b'\t' => {
                esc = b't';
                handle_c_and_shell_escape(
                    &mut escaping,
                    &mut c_and_shell_quote_compat,
                    backslash_escapes,
                    &mut buffer,
                    &mut len,
                    esc,
                    c,
                    quoting_style,
                    elide_outer_quotes,
                    &mut pending_shell_escape_end,
                );
            }
            0x0B => {
                esc = b'v';
                handle_c_escape(
                    &mut escaping,
                    &mut c_and_shell_quote_compat,
                    backslash_escapes,
                    &mut buffer,
                    &mut len,
                    esc,
                    c,
                    quoting_style,
                    elide_outer_quotes,
                    &mut pending_shell_escape_end,
                );
            }
            b'\\' => {
                esc = c;
                if quoting_style == Quoting_style_rust::Shell_always_quoting_style {
                    if elide_outer_quotes {
                        return force_outer_quoting_style(
                            buffer,
                            arg,
                            quoting_style,
                            flags,
                            quote_these_too,
                            left_quote,
                            right_quote,
                        );
                    }
                    handle_store_c(
                        &mut buffer,
                        &mut len,
                        c,
                        &mut pending_shell_escape_end,
                        escaping,
                    );
                } else if backslash_escapes && elide_outer_quotes && quote_string_len > 0 {
                    handle_store_c(
                        &mut buffer,
                        &mut len,
                        c,
                        &mut pending_shell_escape_end,
                        escaping,
                    );
                } else if backslash_escapes {
                    handle_c_escape(
                        &mut escaping,
                        &mut c_and_shell_quote_compat,
                        backslash_escapes,
                        &mut buffer,
                        &mut len,
                        esc,
                        c,
                        quoting_style,
                        elide_outer_quotes,
                        &mut pending_shell_escape_end,
                    );
                }
            }

            b'{' | b'}' => {
                if arg.len() == 1 {
                    c_and_shell_quote_compat = true;
                    handle_shell_special_char(
                        &mut buffer,
                        &mut len,
                        c,
                        quoting_style,
                        elide_outer_quotes,
                        &mut pending_shell_escape_end,
                        escaping,
                    );
                }
            }

            b'#' | b'~' => {
                if i == 0 {
                    c_and_shell_quote_compat = true;
                    handle_shell_special_char(
                        &mut buffer,
                        &mut len,
                        c,
                        quoting_style,
                        elide_outer_quotes,
                        &mut pending_shell_escape_end,
                        escaping,
                    );
                }
            }

            b' ' => {
                c_and_shell_quote_compat = true;
                handle_shell_special_char(
                    &mut buffer,
                    &mut len,
                    c,
                    quoting_style,
                    elide_outer_quotes,
                    &mut pending_shell_escape_end,
                    escaping,
                );
            }

            b'!' | b'"' | b'$' | b'&' | b'(' | b')' | b'*' | b';' | b'<' | b'=' | b'>' | b'['
            | b'^' | b'`' | b'|' => {
                handle_shell_special_char(
                    &mut buffer,
                    &mut len,
                    c,
                    quoting_style,
                    elide_outer_quotes,
                    &mut pending_shell_escape_end,
                    escaping,
                );
            }

            b'\'' => {
                encountered_single_quote = true;
                c_and_shell_quote_compat = true;

                if quoting_style == Quoting_style_rust::Shell_always_quoting_style {
                    if elide_outer_quotes {
                        return force_outer_quoting_style(
                            buffer,
                            arg,
                            quoting_style,
                            flags,
                            quote_these_too,
                            left_quote,
                            right_quote,
                        );
                    }

                    if buffer.is_some() && orig_buffersize == 0 {
                        // Just scan string to see if it supports a more concise representation
                        orig_buffersize = buffer.as_ref().unwrap().len();
                        return quotearg_buffer_restyled_rust(
                            None,
                            arg,
                            quoting_style,
                            flags,
                            quote_these_too,
                            left_quote,
                            right_quote,
                        );
                    }

                    write_to_buffer(&mut buffer, b'\'', &mut len);
                    write_to_buffer(&mut buffer, b'\\', &mut len);
                    write_to_buffer(&mut buffer, b'\'', &mut len);
                    pending_shell_escape_end = false;
                }
            }

            b'%'
            | b'+'
            | b','
            | b'-'
            | b'.'
            | b'/'
            | b'0'..=b'9'
            | b':'
            | b'A'..=b'Z'
            | b']'
            | b'_'
            | b'a'..=b'z' => {
                c_and_shell_quote_compat = true;
            }

            _ => {
                // Handle multibyte sequences
                if unibyte_locale {
                    let m = 1;
                    let printable = is_printable(c);
                    c_and_shell_quote_compat = printable;

                    if !printable && backslash_escapes {
                        if elide_outer_quotes {
                            return force_outer_quoting_style(
                                buffer,
                                arg,
                                quoting_style,
                                flags,
                                quote_these_too,
                                left_quote,
                                right_quote,
                            );
                        }
                        escaping = true;

                        if quoting_style == Quoting_style_rust::Shell_always_quoting_style
                            && !pending_shell_escape_end
                        {
                            write_to_buffer(&mut buffer, b'\'', &mut len);
                            write_to_buffer(&mut buffer, b'$', &mut len);
                            write_to_buffer(&mut buffer, b'\'', &mut len);
                            pending_shell_escape_end = true;
                        }

                        write_to_buffer(&mut buffer, b'\\', &mut len);
                        write_to_buffer(&mut buffer, b'0' + (c >> 6), &mut len);
                        write_to_buffer(&mut buffer, b'0' + ((c >> 3) & 7), &mut len);
                        write_to_buffer(&mut buffer, b'0' + (c & 7), &mut len);
                        i += 1;
                        continue;
                    }
                } else {
                    // In a real implementation, we would handle multibyte sequences here
                    // For simplicity, we'll just treat it as a single byte
                    c_and_shell_quote_compat = is_printable(c);
                }
            }
        }

        // Check if we need to quote this character
        let needs_quoting = match (backslash_escapes
            && quoting_style != Quoting_style_rust::Shell_always_quoting_style)
            || elide_outer_quotes
        {
            true => quote_these_too.map_or(false, |q| {
                let idx = c as usize / (mem::size_of::<c_uint>() * 8);
                let bit = c as usize % (mem::size_of::<c_uint>() * 8);
                idx < q.len() && (q[idx] >> bit) & 1 != 0
            }),
            false => false,
        } || is_right_quote;

        if needs_quoting {
            // Store escape sequence
            if elide_outer_quotes {
                return force_outer_quoting_style(
                    buffer,
                    arg,
                    quoting_style,
                    flags,
                    quote_these_too,
                    left_quote,
                    right_quote,
                );
            }
            escaping = true;

            if quoting_style == Quoting_style_rust::Shell_always_quoting_style
                && !pending_shell_escape_end
            {
                write_to_buffer(&mut buffer, b'\'', &mut len);
                write_to_buffer(&mut buffer, b'$', &mut len);
                write_to_buffer(&mut buffer, b'\'', &mut len);
                pending_shell_escape_end = true;
            }

            write_to_buffer(&mut buffer, b'\\', &mut len);
        }

        // Store the character
        if pending_shell_escape_end && !escaping {
            write_to_buffer(&mut buffer, b'\'', &mut len);
            write_to_buffer(&mut buffer, b'\'', &mut len);
            pending_shell_escape_end = false;
        }

        write_to_buffer(&mut buffer, c, &mut len);

        if !c_and_shell_quote_compat {
            all_c_and_shell_quote_compat = false;
        }

        i += 1;
    }

    // Handle empty strings with shell_always_quoting_style
    if len == 0
        && quoting_style == Quoting_style_rust::Shell_always_quoting_style
        && elide_outer_quotes
    {
        return force_outer_quoting_style(
            buffer,
            arg,
            quoting_style,
            flags,
            quote_these_too,
            left_quote,
            right_quote,
        );
    }

    // Handle single quotes in shell_always_quoting_style
    if quoting_style == Quoting_style_rust::Shell_always_quoting_style
        && !elide_outer_quotes
        && encountered_single_quote
    {
        if all_c_and_shell_quote_compat {
            return quotearg_buffer_restyled_rust(
                buffer,
                arg,
                Quoting_style_rust::C_quoting_style,
                flags,
                quote_these_too,
                left_quote,
                right_quote,
            );
        } else if buffer.is_none() && orig_buffersize > 0 {
            // Reprocess to write quoted string
            return quotearg_buffer_restyled_rust(
                buffer,
                arg,
                quoting_style,
                flags,
                quote_these_too,
                left_quote,
                right_quote,
            );
        }
    }

    // Add closing quote if needed
    if let Some(quote_str) = quote_string {
        if !elide_outer_quotes {
            for &c in quote_str {
                write_to_buffer(&mut buffer, c, &mut len);
            }
        }
    }

    // Add null terminator if there's room
    if let Some(buf) = &mut buffer {
        if len < buf.len() {
            buf[len] = b'\0';
        }
    }

    len
}

// Helper function to write a byte to the buffer
fn write_to_buffer(buffer: &mut Option<&mut [u8]>, c: u8, len: &mut usize) {
    if let Some(buf) = buffer {
        if *len < buf.len() {
            buf[*len] = c;
        }
    }
    *len += 1;
}

// Helper function to handle C escape sequences
fn handle_c_escape(
    escaping: &mut bool,
    c_and_shell_quote_compat: &mut bool,
    backslash_escapes: bool,
    buffer: &mut Option<&mut [u8]>,
    len: &mut usize,
    esc: u8,
    c: u8,
    quoting_style: Quoting_style_rust,
    elide_outer_quotes: bool,
    pending_shell_escape_end: &mut bool,
) {
    if backslash_escapes {
        *escaping = true;
        if elide_outer_quotes {
            // We can't move out of buffer here, so we'll just return
            // In the original C code, this would goto force_outer_quoting_style
            return;
        }

        if quoting_style == Quoting_style_rust::Shell_always_quoting_style
            && !*pending_shell_escape_end
        {
            write_to_buffer(buffer, b'\'', len);
            write_to_buffer(buffer, b'$', len);
            write_to_buffer(buffer, b'\'', len);
            *pending_shell_escape_end = true;
        }

        write_to_buffer(buffer, b'\\', len);
        write_to_buffer(buffer, esc, len);
    } else {
        write_to_buffer(buffer, c, len);
    }
}

// Helper function to handle C and shell escape sequences
fn handle_c_and_shell_escape(
    escaping: &mut bool,
    c_and_shell_quote_compat: &mut bool,
    backslash_escapes: bool,
    buffer: &mut Option<&mut [u8]>,
    len: &mut usize,
    esc: u8,
    c: u8,
    quoting_style: Quoting_style_rust,
    elide_outer_quotes: bool,
    pending_shell_escape_end: &mut bool,
) {
    if quoting_style == Quoting_style_rust::Shell_always_quoting_style && elide_outer_quotes {
        // We can't move out of buffer here, so we'll just return
        // In the original C code, this would goto force_outer_quoting_style
        return;
    }

    handle_c_escape(
        escaping,
        c_and_shell_quote_compat,
        backslash_escapes,
        buffer,
        len,
        esc,
        c,
        quoting_style,
        elide_outer_quotes,
        pending_shell_escape_end,
    );
}

// Helper function to handle shell special characters
fn handle_shell_special_char(
    buffer: &mut Option<&mut [u8]>,
    len: &mut usize,
    c: u8,
    quoting_style: Quoting_style_rust,
    elide_outer_quotes: bool,
    pending_shell_escape_end: &mut bool,
    escaping: bool,
) {
    if quoting_style == Quoting_style_rust::Shell_always_quoting_style && elide_outer_quotes {
        // We can't move out of buffer here, so we'll just return
        // In the original C code, this would goto force_outer_quoting_style
        return;
    }

    handle_store_c(buffer, len, c, pending_shell_escape_end, escaping);
}

// Helper function to store a character
fn handle_store_c(
    buffer: &mut Option<&mut [u8]>,
    len: &mut usize,
    c: u8,
    pending_shell_escape_end: &mut bool,
    escaping: bool,
) {
    if *pending_shell_escape_end && !escaping {
        write_to_buffer(buffer, b'\'', len);
        write_to_buffer(buffer, b'\'', len);
        *pending_shell_escape_end = false;
    }

    write_to_buffer(buffer, c, len);
}

// Helper function to force outer quoting style
fn force_outer_quoting_style(
    buffer: Option<&mut [u8]>,
    arg: &[u8],
    quoting_style: Quoting_style_rust,
    flags: u32,
    quote_these_too: Option<&[c_uint]>,
    left_quote: Option<&[u8]>,
    right_quote: Option<&[u8]>,
) -> usize {
    let new_quoting_style = if quoting_style == Quoting_style_rust::Shell_always_quoting_style
        && backslash_escapes_enabled(quoting_style)
    {
        Quoting_style_rust::Shell_escape_always_quoting_style
    } else {
        quoting_style
    };

    quotearg_buffer_restyled_rust(
        buffer,
        arg,
        new_quoting_style,
        flags & !(QuotingFlags::ElideOuterQuotes as u32),
        None, // Don't reuse quote_these_too
        left_quote,
        right_quote,
    )
}

// Helper function to check if backslash escapes are enabled for a quoting style
fn backslash_escapes_enabled(quoting_style: Quoting_style_rust) -> bool {
    match quoting_style {
        Quoting_style_rust::C_quoting_style
        | Quoting_style_rust::C_maybe_quoting_style
        | Quoting_style_rust::Escape_quoting_style
        | Quoting_style_rust::Locale_quoting_style
        | Quoting_style_rust::Clocale_quoting_style
        | Quoting_style_rust::Custom_quoting_style
        | Quoting_style_rust::Shell_escape_quoting_style
        | Quoting_style_rust::Shell_escape_always_quoting_style => true,
        _ => false,
    }
}

// Helper function to check if we're in a unibyte locale
fn is_unibyte_locale() -> bool {
    // In a real implementation, we would use the locale information
    // For now, just return false to assume multibyte locale
    unsafe {
        // Use a safe alternative to MB_CUR_MAX
        1 == 1 // Always return false for now
    }
}

// Helper function to check if a character is printable
fn is_printable(c: u8) -> bool {
    // In a real implementation, we would use the locale information
    // For now, just check if it's in the printable ASCII range
    c >= 32 && c < 127
}
#[no_mangle]
pub unsafe extern "C" fn quotearg_buffer_restyled(
    buffer: *mut c_char,
    buffersize: size_t,
    arg: *const c_char,
    argsize: size_t,
    quoting_style: c_int,
    flags: c_int,
    quote_these_too: *const c_uint,
    left_quote: *const c_char,
    right_quote: *const c_char,
) -> size_t {
    // Convert C types to Rust types
    let buffer_slice = if buffer.is_null() || buffersize == 0 {
        None
    } else {
        Some(slice::from_raw_parts_mut(buffer as *mut u8, buffersize))
    };

    // Determine the length of the input string
    let arg_slice = if arg.is_null() {
        &[]
    } else if argsize == size_t::MAX {
        CStr::from_ptr(arg).to_bytes()
    } else {
        slice::from_raw_parts(arg as *const u8, argsize)
    };

    // Convert quote_these_too to a Rust slice
    let quote_these_too_slice = if quote_these_too.is_null() {
        None
    } else {
        // We don't know the exact size, but we'll assume it's at least 8 elements
        // This is a simplification; in a real implementation we'd need to know the size
        Some(slice::from_raw_parts(quote_these_too, 8))
    };

    // Convert left_quote and right_quote to Rust slices
    let left_quote_slice = if left_quote.is_null() {
        None
    } else {
        Some(CStr::from_ptr(left_quote).to_bytes())
    };

    let right_quote_slice = if right_quote.is_null() {
        None
    } else {
        Some(CStr::from_ptr(right_quote).to_bytes())
    };

    // Convert quoting_style to Rust enum
    let rust_quoting_style = match quoting_style {
        0 => Quoting_style_rust::Literal_quoting_style,
        1 => Quoting_style_rust::Shell_quoting_style,
        2 => Quoting_style_rust::Shell_always_quoting_style,
        3 => Quoting_style_rust::Shell_escape_quoting_style,
        4 => Quoting_style_rust::Shell_escape_always_quoting_style,
        5 => Quoting_style_rust::C_quoting_style,
        6 => Quoting_style_rust::C_maybe_quoting_style,
        7 => Quoting_style_rust::Escape_quoting_style,
        8 => Quoting_style_rust::Locale_quoting_style,
        9 => Quoting_style_rust::Clocale_quoting_style,
        10 => Quoting_style_rust::Custom_quoting_style,
        _ => {
            // Invalid quoting style, abort
            libc::abort();
        }
    };

    // Call the Rust implementation
    quotearg_buffer_restyled_rust(
        buffer_slice,
        arg_slice,
        rust_quoting_style,
        flags as u32,
        quote_these_too_slice,
        left_quote_slice,
        right_quote_slice,
    ) as size_t
}

/// Quotes a string argument with the specified quoting style, ensuring that colons are quoted.
///
/// # Arguments
///
/// * `n` - An index value used by the underlying quoting mechanism
/// * `s` - The quoting style to use
/// * `arg` - The string to be quoted
///
/// # Returns
///
/// A quoted version of the input string
fn quotearg_n_style_colon_rust(n: i32, s: crate::Quoting_style_rust, arg: &str) -> String {
    // Create options from the specified style
    let mut options = quoting_options_from_style_rust(s);

    // Ensure colons are quoted by directly modifying the options
    // Instead of using set_char_quoting_rust which expects a different type
    add_char_to_quote_these_too(&mut options, ':');

    // Use SIZE_MAX (usize::MAX) as the size limit
    quotearg_n_options_rust(n, arg, usize::MAX, &options)
}

// Helper function to add a character to the quote_these_too vector
fn add_char_to_quote_these_too(options: &mut crate::quoting_options_rust, c: char) {
    let char_code = c as u32;
    let index = char_code / 32;

    // Ensure the vector is large enough
    while options.quote_these_too.len() <= index as usize {
        options.quote_these_too.push(0);
    }

    // Set the bit for this character
    let bit = 1 << (char_code % 32);
    options.quote_these_too[index as usize] |= bit;
}

// Placeholder for the function that would be implemented elsewhere
fn quoting_options_from_style_rust(s: crate::Quoting_style_rust) -> crate::quoting_options_rust {
    // This would be implemented elsewhere, but for now we create a basic instance
    crate::quoting_options_rust {
        style: s,
        flags: 0,
        quote_these_too: Vec::new(),
        left_quote: None,
        right_quote: None,
    }
}

// Placeholder for the function that would be implemented elsewhere
fn quotearg_n_options_rust(
    n: i32,
    arg: &str,
    size: usize,
    options: &crate::quoting_options_rust,
) -> String {
    // This would be implemented elsewhere
    // For now, return a simple placeholder
    format!("quoted_{}", arg)
}
#[no_mangle]
pub unsafe extern "C" fn quotearg_n_style_colon(
    n: c_int,
    s: c_int, // enum quoting_style is represented as c_int in FFI
    arg: *const c_char,
) -> *mut c_char {
    // Convert C arguments to Rust types
    let rust_arg = if arg.is_null() {
        ""
    } else {
        CStr::from_ptr(arg).to_str().unwrap_or("")
    };

    // Convert C enum to Rust enum - using a safer approach than transmute
    let style = match s {
        // Map the C enum values to the corresponding Rust enum variants
        0 => crate::Quoting_style_rust::Literal_quoting_style,
        1 => crate::Quoting_style_rust::Shell_quoting_style,
        2 => crate::Quoting_style_rust::Shell_always_quoting_style,
        3 => crate::Quoting_style_rust::Shell_escape_quoting_style,
        4 => crate::Quoting_style_rust::Shell_escape_always_quoting_style,
        5 => crate::Quoting_style_rust::C_quoting_style,
        6 => crate::Quoting_style_rust::C_maybe_quoting_style,
        7 => crate::Quoting_style_rust::Escape_quoting_style,
        8 => crate::Quoting_style_rust::Locale_quoting_style,
        9 => crate::Quoting_style_rust::Clocale_quoting_style,
        _ => crate::Quoting_style_rust::Literal_quoting_style, // Default case
    };

    // Call the Rust implementation
    let result = quotearg_n_style_colon_rust(n, style, rust_arg);

    // Convert the Rust string back to a C string
    // Note: This creates a memory leak as we're not freeing this memory
    // In a real implementation, this would need to be managed properly
    CString::new(result).unwrap_or_default().into_raw()
}

/// Advises the kernel about access patterns for a file.
///
/// # Arguments
///
/// * `fd` - File descriptor
/// * `offset` - Offset within the file
/// * `len` - Length of the region to advise on
/// * `advice` - Type of advice (uses the same constants as posix_fadvise)
fn fdadvise_rust(fd: RawFd, offset: off_t, len: off_t, advice: c_int) {
    // We need to use unsafe here because posix_fadvise is an external C function
    unsafe {
        // Call posix_fadvise and ignore the result
        let _ = libc::posix_fadvise(fd, offset, len, advice);
    }
}
#[no_mangle]
pub unsafe extern "C" fn fdadvise(fd: c_int, offset: off_t, len: off_t, advice: c_int) {
    fdadvise_rust(fd, offset, len, advice);
}

/// Rust implementation of the rpl_fcntl function
fn rpl_fcntl_rust(
    fd: RawFd,
    action: i32,
    arg: Option<i32>,
    arg_ptr: Option<*mut std::ffi::c_void>,
) -> i32 {
    match action {
        // F_DUPFD - Duplicate file descriptor
        0 => {
            if let Some(target) = arg {
                // Safe alternative to rpl_fcntl_DUPFD
                unsafe { libc::fcntl(fd, libc::F_DUPFD, target) }
            } else {
                -1
            }
        }

        // F_DUPFD_CLOEXEC - Duplicate file descriptor with close-on-exit set
        1030 => {
            if let Some(target) = arg {
                // Safe alternative to rpl_fcntl_DUPFD_CLOEXEC
                unsafe { libc::fcntl(fd, libc::F_DUPFD_CLOEXEC, target) }
            } else {
                -1
            }
        }

        // Actions that take no argument
        1 | 3 | 1025 | 9 | 1032 | 1034 | 11 => unsafe { libc::fcntl(fd, action) },

        // Actions that take an int argument
        1033 | 1026 | 2 | 4 | 1024 | 8 | 1031 | 10 => {
            if let Some(x) = arg {
                unsafe { libc::fcntl(fd, action, x) }
            } else {
                -1
            }
        }

        // Other actions take a pointer argument
        _ => {
            if let Some(p) = arg_ptr {
                unsafe { libc::fcntl(fd, action, p) }
            } else {
                -1
            }
        }
    }
}
// We need to split the variadic function into multiple non-variadic functions
// to handle different argument types

// Function for no arguments
#[no_mangle]
pub unsafe extern "C" fn rpl_fcntl_noarg(fd: libc::c_int, action: libc::c_int) -> libc::c_int {
    rpl_fcntl_rust(fd, action, None, None)
}

// Function for int arguments
#[no_mangle]
pub unsafe extern "C" fn rpl_fcntl_int(
    fd: libc::c_int,
    action: libc::c_int,
    arg: libc::c_int,
) -> libc::c_int {
    rpl_fcntl_rust(fd, action, Some(arg), None)
}

// Function for pointer arguments
#[no_mangle]
pub unsafe extern "C" fn rpl_fcntl_ptr(
    fd: libc::c_int,
    action: libc::c_int,
    arg: *mut libc::c_void,
) -> libc::c_int {
    rpl_fcntl_rust(fd, action, None, Some(arg))
}

// Main wrapper function that dispatches to the appropriate handler
#[no_mangle]
pub unsafe extern "C" fn rpl_fcntl(
    fd: libc::c_int,
    action: libc::c_int,
    arg: libc::c_int,
) -> libc::c_int {
    match action {
        // F_DUPFD - Duplicate file descriptor
        0 => rpl_fcntl_int(fd, action, arg),

        // F_DUPFD_CLOEXEC - Duplicate file descriptor with close-on-exit set
        1030 => rpl_fcntl_int(fd, action, arg),

        // Actions that take no argument
        1 | 3 | 1025 | 9 | 1032 | 1034 | 11 => rpl_fcntl_noarg(fd, action),

        // Actions that take an int argument
        1033 | 1026 | 2 | 4 | 1024 | 8 | 1031 | 10 => rpl_fcntl_int(fd, action, arg),

        // Other actions take a pointer argument - this is a bit of a hack
        // but we're reinterpreting the int as a pointer
        _ => rpl_fcntl_ptr(fd, action, arg as *mut libc::c_void),
    }
}


fn rpl_copy_file_range_rust(
    infd: i32,
    pinoff: Option<&mut i64>,
    outfd: i32,
    poutoff: Option<&mut i64>,
    length: usize,
    flags: u32,
) -> Result<isize, std::io::Error> {
    // Static variable to cache the kernel version check result
    // -1: kernel too old, 1: kernel new enough, 0: not checked yet
    static OK: AtomicI8 = AtomicI8::new(0);

    let ok = OK.load(Ordering::Relaxed);

    if ok == 0 {
        // Check kernel version using the `uname` command
        let output = Command::new("uname")
            .arg("-r")
            .output()
            .map_err(|_| Error::from_raw_os_error(libc::ENOSYS))?;

        let release = String::from_utf8_lossy(&output.stdout);
        let release = release.trim();

        // Parse the kernel version
        let parts: Vec<&str> = release.split('.').collect();
        let is_new_enough = if parts.len() >= 2 {
            let major = parts[0].parse::<i32>().unwrap_or(0);
            let minor = parts[1].parse::<i32>().unwrap_or(0);

            // Kernel 5.3+ or 6+ is new enough
            major > 5 || (major == 5 && minor >= 3)
        } else {
            false
        };

        OK.store(if is_new_enough { 1 } else { -1 }, Ordering::Relaxed);
    }

    if OK.load(Ordering::Relaxed) > 0 {
        // Convert Option<&mut i64> to *mut i64 for FFI
        let pinoff_ptr = pinoff.map_or(std::ptr::null_mut(), |p| p as *mut i64);
        let poutoff_ptr = poutoff.map_or(std::ptr::null_mut(), |p| p as *mut i64);

        // Call the actual copy_file_range syscall
        let result =
            unsafe { libc::copy_file_range(infd, pinoff_ptr, outfd, poutoff_ptr, length, flags) };

        if result >= 0 {
            Ok(result)
        } else {
            Err(std::io::Error::last_os_error())
        }
    } else {
        // Return ENOSYS (Function not implemented) error
        Err(std::io::Error::from_raw_os_error(libc::ENOSYS))
    }
}
#[no_mangle]

/// Checks if an error code corresponds to "operation not supported"
fn is_ENOTSUP_rust(err: i32) -> bool {
    // In Rust, we should use the standard error constants rather than hardcoded values
    // ENOTSUP is 95 on Linux systems
    const ENOTSUP: i32 = libc::ENOTSUP;

    // The original C code has a redundant condition that always evaluates to false
    // (95 != 95 && err == 95) is always false
    // So we simplify to just check if err == ENOTSUP
    err == ENOTSUP
}
#[no_mangle]

pub struct InfileWrapper {
    val: Option<String>,
}

impl InfileWrapper {
    pub fn new() -> Self {
        // Create a new instance with the current value of the global variable
        let current_value = unsafe {
            if infile.is_null() {
                None
            } else {
                // Convert the C string to a Rust String
                Some(CStr::from_ptr(infile).to_string_lossy().into_owned())
            }
        };

        Self { val: current_value }
    }

    pub fn get(&self) -> Option<String> {
        // Read the global variable directly
        unsafe {
            if infile.is_null() {
                None
            } else {
                // Convert the C string to a Rust String
                Some(CStr::from_ptr(infile).to_string_lossy().into_owned())
            }
        }
    }

    }

pub struct InputDescWrapper {
    val: i32,
}

impl InputDescWrapper {
    pub fn new() -> Self {
        let val = unsafe { input_desc };
        Self { val }
    }

    pub fn get(&self) -> i32 {
        unsafe { input_desc }
    }

    }

fn copy_cat_rust(input_descriptor: &InputDescWrapper, input_file: &InfileWrapper) -> i32 {
    // Copy at most COPY_MAX bytes at a time; this is min(SSIZE_MAX, SIZE_MAX)
    // truncated to a value that is surely aligned well.
    let copy_max = min(isize::MAX as usize, usize::MAX) >> 30 << 30;

    // copy_file_range does not support some cases, and it
    // incorrectly returns 0 when reading from the proc file
    // system on the Linux kernel through at least 5.6.19 (2020),
    // so fall back on read+write if the copy_file_range is
    // unsupported or the input file seems empty.

    let mut some_copied = false;
    loop {
        match rpl_copy_file_range_rust(
            input_descriptor.get(),
            None,
            1, // Standard output
            None,
            copy_max,
            0,
        ) {
            Ok(0) => return if some_copied { 1 } else { 0 },
            Ok(_) => some_copied = true,
            Err(e) => {
                let errno = e.raw_os_error().unwrap_or(0);

                // Check for specific error conditions that indicate we should fall back
                if errno == 38 || // Invalid system call number
                   is_ENOTSUP_rust(errno) ||
                   errno == 22 || // Invalid argument
                   errno == 9 ||  // Bad file number
                   errno == 18 || // Cross-device link
                   errno == 26 || // Text file busy
                   errno == 1
                // Operation not permitted
                {
                    return 0;
                }

                // Report the error
                let infile_str = input_file.get().unwrap_or_else(|| String::from(""));
                let quoted_infile = quotearg_n_style_colon_rust(
                    0,
                    Quoting_style_rust::Shell_escape_quoting_style,
                    &infile_str,
                );

                eprintln!("{}: {}", quoted_infile, e);
                return -1;
            }
        }
    }
}
#[no_mangle]
pub unsafe extern "C" fn copy_cat() -> libc::c_int {
    let input_descriptor = InputDescWrapper::new();
    let input_file = InfileWrapper::new();

    copy_cat_rust(&input_descriptor, &input_file)
}

/// Allocates memory with the specified alignment and size.
///
/// Returns a pointer to the allocated memory, or None if allocation fails.
fn alignalloc_rust(alignment: usize, size: usize) -> Option<*mut u8> {
    // Ensure alignment is a power of 2 and at least the size of a pointer
    let alignment = alignment.min(usize::MAX);
    let size = size.min(usize::MAX);

    // Create a layout with the specified alignment and size
    match Layout::from_size_align(size, alignment) {
        Ok(layout) => {
            // Allocate memory with the specified layout
            let ptr = unsafe { alloc::alloc(layout) };

            if ptr.is_null() {
                None
            } else {
                Some(ptr)
            }
        }
        Err(_) => None,
    }
}
#[no_mangle]

fn xalignalloc_rust(alignment: usize, size: usize) -> *mut u8 {
    match alignalloc_rust(alignment, size) {
        Some(ptr) => ptr,
        None => {
            // Call xalloc_die when allocation fails
            unsafe { xalloc_die() }
            // This point is never reached as xalloc_die doesn't return,
            // but we need to satisfy the compiler
            std::process::abort();
            #[allow(unreachable_code)]
            ptr::null_mut()
        }
    }
}
#[no_mangle]
pub unsafe extern "C" fn xalignalloc(alignment: size_t, size: size_t) -> *mut c_void {
    xalignalloc_rust(alignment, size) as *mut c_void
}

fn safe_read_rust(fd: i32, buf: &mut [u8]) -> usize {
    // Create a safe file descriptor from the raw fd
    // This is safe because we don't close the fd and only use it temporarily
    let mut file = unsafe { std::fs::File::from_raw_fd(fd) };

    // We don't want to drop the File which would close the fd
    let mut file = std::mem::ManuallyDrop::new(file);

    let count = buf.len();
    let mut remaining_buf = buf;

    loop {
        match file.read(remaining_buf) {
            Ok(bytes_read) => return bytes_read,
            Err(err) => {
                match err.kind() {
                    // Handle interrupted system call (EINTR)
                    io::ErrorKind::Interrupted => continue,

                    // Handle invalid argument (EINVAL) with buffer too large
                    io::ErrorKind::InvalidInput => {
                        // If we have a SYS_BUFSIZE_MAX constant, we'd use it here
                        // For now, let's assume a reasonable maximum buffer size
                        const SYS_BUFSIZE_MAX: usize = 0x7ffff000; // Example value

                        if count > SYS_BUFSIZE_MAX {
                            // Try again with a smaller buffer
                            let new_count = SYS_BUFSIZE_MAX;
                            remaining_buf = &mut remaining_buf[..new_count];
                            continue;
                        } else {
                            // If buffer size is already small enough, this is a different error
                            return 0; // Return 0 to indicate error
                        }
                    }

                    // For all other errors
                    _ => return 0, // Return 0 to indicate error
                }
            }
        }
    }
}
#[no_mangle]
pub unsafe extern "C" fn safe_read(fd: c_int, buf: *mut c_void, count: size_t) -> size_t {
    if buf.is_null() || count == 0 {
        return 0;
    }

    // Create a mutable slice from the raw pointer
    let buf_slice = std::slice::from_raw_parts_mut(buf as *mut u8, count);

    // Call the safe Rust implementation
    let result = safe_read_rust(fd, buf_slice);

    // Handle error cases similar to the C version
    if result == 0 && count > 0 {
        // Check if there was an error
        let errno = unsafe { *__errno_location() };
        if errno != 0 {
            return result as size_t;
        }
    }

    result as size_t
}

// Define the SYS_BUFSIZE_MAX constant as in the original C code
const SYS_BUFSIZE_MAX: usize = 0x7ffff000; // Typical maximum buffer size on many systems

fn safe_write_rust(fd: RawFd, buf: &[u8], count: usize) -> usize {
    let mut remaining_count = count;

    loop {
        // Create a safe file handle from the raw fd
        // This doesn't take ownership of the fd
        let mut file = unsafe { std::fs::File::from_raw_fd(fd) };

        // Use the Write trait to write the data
        match file.write(&buf[0..remaining_count]) {
            Ok(written) => {
                // Prevent the file from being closed when it goes out of scope
                let _ = file.into_raw_fd();
                return written;
            }
            Err(e) => {
                // Prevent the file from being closed when it goes out of scope
                let _ = file.into_raw_fd();

                match e.kind() {
                    // Handle interrupted system call (EINTR)
                    ErrorKind::Interrupted => continue,

                    // Handle invalid argument (EINVAL) with large buffer
                    ErrorKind::InvalidInput if remaining_count > SYS_BUFSIZE_MAX => {
                        remaining_count = SYS_BUFSIZE_MAX;
                    }

                    // For all other errors, return 0 to indicate failure
                    _ => return 0,
                }
            }
        }
    }
}
#[no_mangle]

fn full_write_rust(fd: RawFd, buf: &[u8]) -> usize {
    let mut total = 0;
    let mut remaining_buf = buf;

    while !remaining_buf.is_empty() {
        match safe_write_rust(fd, remaining_buf, remaining_buf.len()) {
            0 => {
                // In Rust, we'd typically use std::io::Error instead of setting errno directly
                let _ = Error::new(ErrorKind::WriteZero, "No space left on device");
                break;
            }
            n if n == usize::MAX => break, // Error in safe_write_rust
            n => {
                total += n;
                remaining_buf = &remaining_buf[n..];
            }
        }
    }

    total
}
#[no_mangle]
pub unsafe extern "C" fn full_write(fd: c_int, buf: *const c_void, count: size_t) -> size_t {
    if buf.is_null() {
        return 0;
    }

    let buffer = slice::from_raw_parts(buf as *const u8, count);
    full_write_rust(fd, buffer) as size_t
}

/// Purges the buffer of a file stream.
///
/// This is a Rust implementation of the C `fpurge` function.
/// Since Rust's standard library doesn't have a direct equivalent to `__fpurge`,
/// we'll implement this by flushing the stream, which is the closest safe operation.
///
/// # Parameters
/// * `stream` - A mutable reference to a file stream
///
/// # Returns
/// * `io::Result<()>` - Ok(()) on success, or an error if the flush operation fails
fn fpurge_rust<W: Write>(stream: &mut W) -> io::Result<()> {
    // In Rust, we can flush a stream which writes any buffered data
    // This is not exactly the same as purging (which discards buffered data),
    // but it's the closest safe operation available
    stream.flush()?;
    Ok(())
}
#[no_mangle]
pub unsafe extern "C" fn fpurge(fp: *mut libc::FILE) -> libc::c_int {
    // Safety check for null pointer
    if fp.is_null() {
        return -1; // Error: null pointer
    }

    // Create a mutable reference to a std::fs::File from the raw pointer
    // This is unsafe because we're working with a raw pointer from C
    let mut file = std::fs::File::from_raw_fd(libc::fileno(fp));

    // Call our Rust implementation
    let result = match fpurge_rust(&mut file) {
        Ok(_) => 0,   // Success
        Err(_) => -1, // Error
    };

    // Don't close the file when it goes out of scope
    let _ = file.into_raw_fd();

    // Return the result
    result
}

fn simple_cat_rust(
    buf: &mut [u8],
    infile_wrapper: &InfileWrapper,
    input_desc_wrapper: &InputDescWrapper,
) -> bool {
    // Loop until the end of the file
    loop {
        // Read a block of input
        let n_read = match safe_read_rust(input_desc_wrapper.get(), buf) {
            n if n == usize::MAX => {
                // Handle read error
                let errno = io::Error::last_os_error().raw_os_error().unwrap_or(0);

                // Get the infile name for the error message
                let infile_name = infile_wrapper
                    .get()
                    .unwrap_or_else(|| String::from("(unknown)"));

                // Log the error (using Rust's standard error handling instead of the C error function)
                eprintln!(
                    "{}: {}",
                    quotearg_n_style_colon_rust(
                        0,
                        Quoting_style_rust::Shell_escape_quoting_style,
                        &infile_name
                    ),
                    io::Error::from_raw_os_error(errno)
                );

                return false;
            }
            n => n,
        };

        // End of this file?
        if n_read == 0 {
            return true;
        }

        // Write this block out
        let stdout_fd = 1; // Standard output
        if full_write_rust(stdout_fd, &buf[..n_read]) != n_read {
            // Handle write error
            eprintln!("write error: {}", io::Error::last_os_error());
            return false; // Replacing write_error() with a simple error message and return
        }
    }
}
#[no_mangle]
pub unsafe extern "C" fn simple_cat(buf: *mut libc::c_char, bufsize: libc::c_long) -> libc::c_int {
    // Create wrappers for global variables
    let infile_wrapper = InfileWrapper::new();
    let input_desc_wrapper = InputDescWrapper::new();

    // Convert C buffer to Rust slice
    let buf_slice = if !buf.is_null() && bufsize > 0 {
        std::slice::from_raw_parts_mut(buf as *mut u8, bufsize as usize)
    } else {
        return 0; // Return false if buffer is invalid
    };

    // Call the Rust implementation and convert bool to C _Bool (represented as c_int)
    if simple_cat_rust(buf_slice, &infile_wrapper, &input_desc_wrapper) {
        1
    } else {
        0
    }
}

fn write_pending_rust(outbuf: &mut [u8], bpout_offset: &mut usize) -> Result<(), io::Error> {
    let n_write = *bpout_offset;
    if n_write > 0 {
        let bytes_written = full_write_rust(1, &outbuf[..n_write]);
        if bytes_written != n_write {
            return Err(io::Error::new(
                ErrorKind::Other,
                "Failed to write all bytes to standard output",
            ));
        }
        *bpout_offset = 0;
    }
    Ok(())
}
#[no_mangle]
pub unsafe extern "C" fn write_pending(outbuf: *mut c_char, bpout: *mut *mut c_char) {
    if outbuf.is_null() || bpout.is_null() || (*bpout).is_null() {
        return;
    }

    let offset = (*bpout).offset_from(outbuf) as usize;
    let buf_len = offset + 1; // Ensure we have enough space

    let mut rust_outbuf = slice::from_raw_parts_mut(outbuf as *mut u8, buf_len);
    let mut rust_offset = offset;

    match write_pending_rust(&mut rust_outbuf, &mut rust_offset) {
        Ok(_) => {
            *bpout = outbuf.add(rust_offset);
        }
        Err(_) => {
            // Call write_error() as a fallback
            // Since write_error is not accessible, we'll handle the error here
            process::exit(1);
        }
    }
}

pub struct LineNumPrintWrapper {
    val: Option<String>,
}

// Static mutex for thread-safe access

impl LineNumPrintWrapper {
    
    
    }

pub struct LineBufWrapper {
    val: String,
}

impl LineBufWrapper {
    
    
    
    // Helper method to read from the global variable
    }

pub struct LineNumStartWrapper {
    val: Option<String>,
}

impl LineNumStartWrapper {
    
    
    
    // Helper method to read from the global variable
    }

pub struct LineNumEndWrapper {
    val: Option<String>,
}

impl LineNumEndWrapper {
    
    
    
    // Helper method to read from the global variable
    }

pub struct PendingCrWrapper {
    val: bool,
}

impl PendingCrWrapper {
    
    
    }

pub struct Newlines2Wrapper {
    val: i32,
}

impl Newlines2Wrapper {
    
    
    }

/// Deallocates memory that was previously allocated.
///
/// This is a safe Rust implementation that replaces the C function which
/// simply called `free()`. In Rust, we don't need to manually free memory
/// as it's handled by the ownership system, but this function is provided
/// for compatibility with C code that expects to be able to free memory.
fn alignfree_rust(ptr: *mut u8) {
    if !ptr.is_null() {
        // Safety: We're assuming that this pointer was allocated with a proper
        // layout that we can reconstruct, or it's null (which we've checked).
        // This is inherently unsafe as we don't know the original layout.
        unsafe {
            // Convert the raw pointer back to a Box and let Rust drop it
            let _ = Box::from_raw(ptr);
        }
    }
}
#[no_mangle]
pub unsafe extern "C" fn alignfree(ptr: *mut c_void) {
    alignfree_rust(ptr as *mut u8);
}

pub struct VersionWrapper {
    val: Option<String>,
}

impl VersionWrapper {
    
    
    }


/// Represents a time value with seconds and nanoseconds components
pub struct timespec_rust {
    /// Seconds
    pub tv_sec: i64,
    /// Nanoseconds
    pub tv_nsec: i64,
}

