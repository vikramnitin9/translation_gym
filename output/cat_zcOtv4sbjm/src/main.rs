#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

#[cfg(target_os = "windows")]
use libc::c_int;
use libc::{
    self, c_char, c_int, c_uint, c_ulong, c_ulonglong, free as libc_free, off_t as libc_off_t,
    size_t, ssize_t, uname as libc_uname, utsname as libc_utsname, ENOSYS as LIBC_ENOSYS,
    ENOTSUP as LIBC_ENOTSUP, EOPNOTSUPP as LIBC_EOPNOTSUPP,
};
use std::os::fd::FromRawFd; // Import the trait for `from_raw_fd`
use std::{
    alloc::{alloc, Layout},
    cmp::{min, Ordering},
    env,
    ffi::{CStr, CString},
    io::{self, Write},
    mem,
    os::raw::{c_uchar, c_void},
    process, ptr,
    ptr::NonNull,
    slice, str,
    sync::{
        atomic::{AtomicI8, Ordering as AtomicOrdering},
        Once,
    },
};
include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
include!("main_func.rs");
fn set_program_name_rust(argv0: &CStr) {
    // Global variables assumed to be defined elsewhere
    extern "C" {
        static mut program_name: *const c_char;
        #[cfg(HAVE_DECL_PROGRAM_INVOCATION_NAME)]
        static mut program_invocation_name: *const c_char;
        #[cfg(HAVE_DECL_PROGRAM_INVOCATION_SHORT_NAME)]
        static mut program_invocation_short_name: *const c_char;
    }

    // Convert CStr to Rust string slice for safe manipulation
    let argv0_str = match argv0.to_str() {
        Ok(s) => s,
        Err(_) => {
            eprintln!("Invalid UTF-8 in argv[0].");
            process::abort(); // Explicitly qualify `abort` to avoid conflicts
        }
    };

    // Find the last slash in the path
    let slash_pos = argv0_str.rfind('/');
    let base = if let Some(pos) = slash_pos {
        &argv0_str[pos + 1..]
    } else {
        argv0_str
    };

    // Check for the "/.libs/" prefix
    let argv0_trimmed =
        if argv0_str.len() >= 7 && argv0_str[slash_pos.unwrap_or(0)..].starts_with("/.libs/") {
            let base_trimmed = if base.starts_with("lt-") {
                &base[3..] // Remove "lt-" prefix
            } else {
                base
            };

            #[cfg(HAVE_DECL_PROGRAM_INVOCATION_SHORT_NAME)]
            unsafe {
                program_invocation_short_name = CString::new(base_trimmed).unwrap().into_raw();
            }

            base_trimmed
        } else {
            argv0_str
        };

    // Update the global variables
    unsafe {
        program_name = CString::new(argv0_trimmed).unwrap().into_raw();

        #[cfg(HAVE_DECL_PROGRAM_INVOCATION_NAME)]
        {
            program_invocation_name = CString::new(argv0_trimmed).unwrap().into_raw();
        }
    }
}
#[no_mangle]
pub extern "C" fn set_program_name(argv0: *const c_char) {
    if argv0.is_null() {
        eprintln!("A NULL argv[0] was passed through an exec system call.");
        process::abort(); // Explicitly qualify `abort` to avoid conflicts
    }

    // Convert the raw C string pointer to a Rust CStr
    let argv0_cstr = unsafe { CStr::from_ptr(argv0) };

    // Call the Rust implementation
    set_program_name_rust(argv0_cstr);
}

static INIT: Once = Once::new();
static mut RESULT_BUF: Option<String> = None;

fn locale_charset_rust() -> &'static str {
    // Initialize the static buffer safely
    unsafe {
        INIT.call_once(|| {
            RESULT_BUF = Some(String::new());
        });
    }

    let mut codeset = None;

    // Check environment variables for locale information
    let locale = env::var("LC_ALL")
        .or_else(|_| env::var("LC_CTYPE"))
        .or_else(|_| env::var("LANG"))
        .unwrap_or_else(|_| String::new());

    if !locale.is_empty() {
        if let Some(dot_pos) = locale.find('.') {
            let encoding = &locale[dot_pos + 1..];
            if let Some(at_pos) = encoding.find('@') {
                let encoding = &encoding[..at_pos];
                unsafe {
                    if let Some(ref mut result_buf) = RESULT_BUF {
                        *result_buf = encoding.to_string();
                        return result_buf.as_str();
                    }
                }
            } else {
                unsafe {
                    if let Some(ref mut result_buf) = RESULT_BUF {
                        *result_buf = encoding.to_string();
                        return result_buf.as_str();
                    }
                }
            }
        }

        if locale == "C" || locale == "POSIX" {
            codeset = Some("");
        }
    }

    if codeset.is_none() {
        // Default to ASCII if no codeset is determined
        codeset = Some("ASCII");
    }

    unsafe {
        if let Some(ref mut result_buf) = RESULT_BUF {
            *result_buf = codeset.unwrap().to_string();
            return result_buf.as_str();
        }
    }

    "ASCII" // Fallback in case RESULT_BUF is not initialized (shouldn't happen)
}
#[no_mangle]
pub extern "C" fn locale_charset() -> *const c_char {
    let result = locale_charset_rust();
    CString::new(result).unwrap().into_raw()
}

fn c_tolower_rust(c: c_int) -> c_int {
    if (c >= 'A' as c_int) && (c <= 'Z' as c_int) {
        c + ('a' as c_int - 'A' as c_int)
    } else {
        c
    }
}
#[no_mangle]
pub extern "C" fn c_tolower(c: c_int) -> c_int {
    c_tolower_rust(c)
}

fn c_strcasecmp_rust(s1: *const c_char, s2: *const c_char) -> c_int {
    // Convert raw C strings to Rust slices
    let s1 = unsafe { CStr::from_ptr(s1) };
    let s2 = unsafe { CStr::from_ptr(s2) };

    // Convert to byte slices for comparison
    let p1 = s1.to_bytes();
    let p2 = s2.to_bytes();

    // If the pointers are the same, the strings are equal
    if p1 == p2 {
        return 0;
    }

    // Compare the strings byte by byte, ignoring case
    for (&b1, &b2) in p1.iter().zip(p2.iter()) {
        let c1 = c_tolower_rust(b1 as c_int) as c_uchar;
        let c2 = c_tolower_rust(b2 as c_int) as c_uchar;

        if c1 != c2 {
            return if c1 < c2 { -1 } else { 1 };
        }

        // If we reach a null terminator, stop comparison
        if c1 == 0 {
            break;
        }
    }

    // Handle cases where one string is a prefix of the other
    match p1.len().cmp(&p2.len()) {
        Ordering::Less => -1,
        Ordering::Greater => 1,
        Ordering::Equal => 0,
    }
}
#[no_mangle]
pub extern "C" fn c_strcasecmp(s1: *const c_char, s2: *const c_char) -> c_int {
    c_strcasecmp_rust(s1, s2)
}

fn proper_name_lite_rust<'a>(name_ascii: &'a str, name_utf8: &'a str) -> &'a str {
    // Call gettext to get the translation
    let translation = gettext_rust(name_ascii);

    // If the translation is different from the input ASCII name, return the translation
    if translation != name_ascii {
        return translation;
    }

    // Check if the locale charset is UTF-8
    if locale_charset_rust().eq_ignore_ascii_case("UTF-8") {
        // If UTF-8, return the UTF-8 name
        return name_utf8;
    }

    // Otherwise, return the ASCII name
    name_ascii
}

// Mock implementation of gettext_rust for demonstration purposes
// Replace this with the actual implementation in your codebase
fn gettext_rust(input: &str) -> &str {
    input // For now, just return the input as the "translation"
}
#[no_mangle]
pub extern "C" fn proper_name_lite(
    name_ascii: *const c_char,
    name_utf8: *const c_char,
) -> *const c_char {
    // Convert the C strings to Rust strings
    let name_ascii = unsafe {
        assert!(!name_ascii.is_null());
        CStr::from_ptr(name_ascii).to_str().unwrap_or("")
    };

    let name_utf8 = unsafe {
        assert!(!name_utf8.is_null());
        CStr::from_ptr(name_utf8).to_str().unwrap_or("")
    };

    // Call the Rust function
    let result = proper_name_lite_rust(name_ascii, name_utf8);

    // Convert the Rust string back to a C string
    CString::new(result).unwrap().into_raw()
}

fn version_etc_arn_rust(
    stream: &mut dyn Write,
    command_name: Option<&str>,
    package: &str,
    version: &str,
    authors: &[&str],
) -> io::Result<()> {
    if let Some(cmd_name) = command_name {
        writeln!(stream, "{} ({}) {}", cmd_name, package, version)?;
    } else {
        writeln!(stream, "{} {}", package, version)?;
    }

    #[cfg(feature = "packager")]
    {
        #[cfg(feature = "packager_version")]
        writeln!(
            stream,
            "Packaged by {} ({})",
            env!("PACKAGE_PACKAGER"),
            env!("PACKAGE_PACKAGER_VERSION")
        )?;
        #[cfg(not(feature = "packager_version"))]
        writeln!(stream, "Packaged by {}", env!("PACKAGE_PACKAGER"))?;
    }

    // Use runtime environment variable access for COPYRIGHT_YEAR
    if let Ok(copyright_year) = std::env::var("COPYRIGHT_YEAR") {
        writeln!(stream, "(C) {}", copyright_year)?;
    } else {
        writeln!(stream, "(C) <unknown year>")?;
    }

    writeln!(stream)?;

    writeln!(
        stream,
        "License GPLv3+: GNU GPL version 3 or later <https://gnu.org/licenses/gpl.html>.\n\
        This is free software: you are free to change and redistribute it.\n\
        There is NO WARRANTY, to the extent permitted by law.\n"
    )?;
    writeln!(stream)?;

    match authors.len() {
        0 => {}
        1 => writeln!(stream, "Written by {}.", authors[0])?,
        2 => writeln!(stream, "Written by {} and {}.", authors[0], authors[1])?,
        3 => writeln!(
            stream,
            "Written by {}, {}, and {}.",
            authors[0], authors[1], authors[2]
        )?,
        4 => writeln!(
            stream,
            "Written by {}, {}, {},\nand {}.",
            authors[0], authors[1], authors[2], authors[3]
        )?,
        5 => writeln!(
            stream,
            "Written by {}, {}, {},\n{}, and {}.",
            authors[0], authors[1], authors[2], authors[3], authors[4]
        )?,
        6 => writeln!(
            stream,
            "Written by {}, {}, {},\n{}, {}, and {}.",
            authors[0], authors[1], authors[2], authors[3], authors[4], authors[5]
        )?,
        7 => writeln!(
            stream,
            "Written by {}, {}, {},\n{}, {}, {}, and {}.",
            authors[0], authors[1], authors[2], authors[3], authors[4], authors[5], authors[6]
        )?,
        8 => writeln!(
            stream,
            "Written by {}, {}, {},\n{}, {}, {}, {},\nand {}.",
            authors[0],
            authors[1],
            authors[2],
            authors[3],
            authors[4],
            authors[5],
            authors[6],
            authors[7]
        )?,
        9 => writeln!(
            stream,
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
        )?,
        _ => writeln!(
            stream,
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
        )?,
    }

    Ok(())
}
#[no_mangle]
pub extern "C" fn version_etc_arn(
    stream: *mut c_void,
    command_name: *const libc::c_char,
    package: *const libc::c_char,
    version: *const libc::c_char,
    authors: *const *const libc::c_char,
    n_authors: usize,
) {
    // Safety: Ensure all pointers are valid and not null
    if stream.is_null() || package.is_null() || version.is_null() {
        return;
    }

    let package = unsafe { CStr::from_ptr(package) }
        .to_str()
        .unwrap_or_default();
    let version = unsafe { CStr::from_ptr(version) }
        .to_str()
        .unwrap_or_default();
    let command_name = if command_name.is_null() {
        None
    } else {
        Some(
            unsafe { CStr::from_ptr(command_name) }
                .to_str()
                .unwrap_or_default(),
        )
    };

    let authors = if authors.is_null() {
        Vec::new()
    } else {
        unsafe { slice::from_raw_parts(authors, n_authors) }
            .iter()
            .map(|&author| {
                unsafe { CStr::from_ptr(author) }
                    .to_str()
                    .unwrap_or_default()
            })
            .collect::<Vec<_>>()
    };

    // Convert the raw stream pointer to a Rust-compatible writer
    let fd = unsafe { libc::fileno(stream as *mut libc::FILE) };
    if fd < 0 {
        return;
    }

    let mut writer = unsafe { std::fs::File::from_raw_fd(fd) };

    let _ = version_etc_arn_rust(&mut writer, command_name, package, version, &authors);
}

fn version_etc_rust(
    stream: *mut _IO_FILE, // Use the correct FILE type from the bindings
    command_name: *const c_char,
    package: *const c_char,
    version: *const c_char,
    authors: *const *const c_char,
) {
    // Convert the null-terminated array of authors into a Vec<*const c_char>
    let mut authors_vec = Vec::new();
    unsafe {
        if !authors.is_null() {
            let mut current = authors;
            while !(*current).is_null() {
                authors_vec.push(*current);
                current = current.add(1);
            }
        }
    }

    // SAFETY: We are calling a C function and ensuring the arguments are valid.
    unsafe {
        version_etc_va(
            stream,
            command_name,
            package,
            version,
            authors_vec.as_ptr() as *mut _,
        );
    }
}
#[no_mangle]
pub extern "C" fn version_etc(
    stream: *mut _IO_FILE, // Use the correct FILE type from the bindings
    command_name: *const c_char,
    package: *const c_char,
    version: *const c_char,
    authors: *const *const c_char,
) {
    version_etc_rust(stream, command_name, package, version, authors);
}

fn io_blksize_rust(st: &libc::stat) -> c_ulonglong {
    const IO_BUFSIZE: c_ulonglong = 8192; // Replace with the actual value of IO_BUFSIZE
    const IDX_MAX: c_ulonglong = c_ulonglong::MAX; // Replace with the actual value of IDX_MAX
    const SIZE_MAX: c_ulonglong = c_ulong::MAX as c_ulonglong; // Replace with the actual value of SIZE_MAX

    // Helper function to calculate the number of leading zeros in a u64
    fn leading_zeros(value: c_ulonglong) -> u32 {
        value.leading_zeros()
    }

    // Treat impossible blocksizes as if they were IO_BUFSIZE
    let mut blocksize = if st.st_blksize <= 0 {
        IO_BUFSIZE
    } else {
        st.st_blksize as c_ulonglong
    };

    // Use a blocksize of at least IO_BUFSIZE bytes, keeping it a multiple of the original blocksize
    blocksize += (IO_BUFSIZE - 1) - (IO_BUFSIZE - 1) % blocksize;

    // For regular files, adjust blocksize if it's not a power of two
    if (st.st_mode & libc::S_IFMT as u32) == libc::S_IFREG as u32
        && (blocksize & (blocksize - 1)) != 0
    {
        let leading_zeros = leading_zeros(blocksize);
        if IDX_MAX < c_ulonglong::MAX || leading_zeros > 0 {
            let power = 1u64 << (64 - leading_zeros);
            if power <= IDX_MAX {
                blocksize = power;
            }
        }
    }

    // Don’t go above the largest power of two that fits in idx_t and size_t
    min(min(IDX_MAX, SIZE_MAX) / 2 + 1, blocksize)
}
#[no_mangle]
pub extern "C" fn io_blksize(st: *const libc::stat) -> c_ulonglong {
    if st.is_null() {
        return 0; // Handle null pointer gracefully
    }

    // Safe dereference of the stat pointer
    let st_ref = unsafe { &*st };
    io_blksize_rust(st_ref)
}

fn __gl_setmode_rust(_fd: libc::c_int, _mode: libc::c_int) -> libc::c_int {
    #[cfg(target_os = "windows")]
    {
        // O_BINARY is used on Windows to set binary mode for file descriptors.
        0x8000 as c_int // O_BINARY value on Windows
    }
    #[cfg(not(target_os = "windows"))]
    {
        // On non-Windows platforms, O_BINARY is not used and can be safely ignored.
        0
    }
}
#[no_mangle]
pub extern "C" fn __gl_setmode(fd: libc::c_int, mode: libc::c_int) -> libc::c_int {
    __gl_setmode_rust(fd, mode)
}

fn set_binary_mode_rust(fd: c_int, mode: c_int) -> c_int {
    __gl_setmode_rust(fd, mode)
}
#[no_mangle]
pub extern "C" fn set_binary_mode(fd: c_int, mode: c_int) -> c_int {
    set_binary_mode_rust(fd, mode)
}

fn xset_binary_mode_rust(fd: libc::c_int, mode: libc::c_int) {
    if set_binary_mode_rust(fd, mode) < 0 {
        // Substitute for xset_binary_mode_error: terminate the process with an error code
        eprintln!("Failed to set binary mode for file descriptor {}", fd);
        process::exit(1);
    }
}
#[no_mangle]
pub extern "C" fn xset_binary_mode(fd: libc::c_int, mode: libc::c_int) {
    xset_binary_mode_rust(fd, mode);
}

fn quotearg_buffer_restyled_rust(
    buffer: &mut [u8],
    arg: &[u8],
    quoting_style: c_int,
    flags: c_int,
    quote_these_too: Option<&[u8]>,
    left_quote: Option<&str>,
    right_quote: Option<&str>,
) -> usize {
    let mut len = 0;
    let mut backslash_escapes = false;

    // Placeholder for the constant QA_ELIDE_OUTER_QUOTES
    let qa_elide_outer_quotes = 1; // Replace with the actual value
    let mut elide_outer_quotes = (flags & qa_elide_outer_quotes) != 0;

    let mut encountered_single_quote = false;

    // Placeholder for the constant SHELL_QUOTING_STYLE
    let shell_quoting_style = 2; // Replace with the actual value

    let left_quote = left_quote.unwrap_or("\"");
    let right_quote = right_quote.unwrap_or("\"");

    match quoting_style {
        // Placeholder for the constant C_QUOTING_STYLE
        1 => {
            if !elide_outer_quotes {
                if len < buffer.len() {
                    buffer[len] = b'"';
                }
                len += 1;
            }
            backslash_escapes = true;
        }
        _ if quoting_style == shell_quoting_style => {
            if !elide_outer_quotes {
                if len < buffer.len() {
                    buffer[len] = b'\'';
                }
                len += 1;
            }
        }
        _ => {}
    }

    for &c in arg {
        match c {
            b'\0' => {
                // Placeholder for the constant QA_ELIDE_NULL_BYTES
                let qa_elide_null_bytes = 4; // Replace with the actual value
                if backslash_escapes {
                    if len + 2 < buffer.len() {
                        buffer[len] = b'\\';
                        buffer[len + 1] = b'0';
                    }
                    len += 2;
                } else if flags & qa_elide_null_bytes != 0 {
                    continue;
                }
            }
            b'\'' => {
                encountered_single_quote = true;
                if quoting_style == shell_quoting_style {
                    if len + 3 < buffer.len() {
                        buffer[len] = b'\'';
                        buffer[len + 1] = b'\\';
                        buffer[len + 2] = b'\'';
                    }
                    len += 3;
                }
            }
            _ => {
                if len < buffer.len() {
                    buffer[len] = c;
                }
                len += 1;
            }
        }
    }

    if !elide_outer_quotes {
        for &q in right_quote.as_bytes() {
            if len < buffer.len() {
                buffer[len] = q;
            }
            len += 1;
        }
    }

    if len < buffer.len() {
        buffer[len] = b'\0';
    }

    len
}
#[no_mangle]
pub extern "C" fn quotearg_buffer_restyled(
    buffer: *mut c_char,
    buffersize: c_ulong,
    arg: *const c_char,
    argsize: c_ulong,
    quoting_style: c_int,
    flags: c_int,
    quote_these_too: *const c_uint,
    left_quote: *const c_char,
    right_quote: *const c_char,
) -> c_ulong {
    unsafe {
        let buffer = std::slice::from_raw_parts_mut(buffer as *mut u8, buffersize as usize);
        let arg = if argsize == usize::MAX as c_ulong {
            CStr::from_ptr(arg).to_bytes()
        } else {
            std::slice::from_raw_parts(arg as *const u8, argsize as usize)
        };
        let left_quote = if left_quote.is_null() {
            None
        } else {
            Some(CStr::from_ptr(left_quote).to_str().unwrap_or_default())
        };
        let right_quote = if right_quote.is_null() {
            None
        } else {
            Some(CStr::from_ptr(right_quote).to_str().unwrap_or_default())
        };
        let quote_these_too = if quote_these_too.is_null() {
            None
        } else {
            Some(std::slice::from_raw_parts(
                quote_these_too as *const u8,
                256, // Assuming 256 for simplicity; adjust as needed
            ))
        };

        quotearg_buffer_restyled_rust(
            buffer,
            arg,
            quoting_style,
            flags,
            quote_these_too,
            left_quote,
            right_quote,
        ) as c_ulong
    }
}

fn quotearg_n_style_colon_rust(
    n: libc::c_int,
    s: libc::c_int, // Assuming `enum quoting_style` is represented as `c_int` in Rust
    arg: *const libc::c_char,
) -> *mut libc::c_char {
    // Substitute for `quoting_options_from_style`
    fn quoting_options_from_style(s: libc::c_int) -> quoting_options {
        // Create a default quoting_options struct and configure it based on `s`
        let mut options: quoting_options = unsafe { mem::zeroed() };
        // Since `_unused` is a placeholder field of type `[u8; 0]`, we cannot store `s` directly.
        // Instead, we assume `s` is used in some other way to configure the options.
        // Placeholder logic: no direct assignment to `_unused`.
        options
    }

    // Substitute for `quotearg_n_options`
    fn quotearg_n_options(
        n: libc::c_int,
        arg: *const libc::c_char,
        size_max: libc::size_t,
        options: &quoting_options,
    ) -> *mut libc::c_char {
        // Placeholder implementation for `quotearg_n_options`
        // In a real implementation, this would perform quoting based on the options
        if arg.is_null() {
            return ptr::null_mut();
        }

        // Convert the input C string to a Rust string
        let c_str = unsafe { CStr::from_ptr(arg) };
        let arg_str = match c_str.to_str() {
            Ok(s) => s,
            Err(_) => return ptr::null_mut(),
        };

        // Perform quoting (this is a placeholder logic)
        let quoted = format!("\"{}\"", arg_str);

        // Convert the quoted string back to a C string
        match CString::new(quoted) {
            Ok(c_string) => c_string.into_raw(),
            Err(_) => ptr::null_mut(),
        }
    }

    // Create quoting options from the style
    let mut options = quoting_options_from_style(s);

    // Set character quoting for ':'
    unsafe {
        set_char_quoting(&mut options as *mut quoting_options, b':' as c_char, 1);
    }

    // Call the substitute for `quotearg_n_options`
    quotearg_n_options(n, arg, libc::size_t::MAX, &options)
}
#[no_mangle]
pub extern "C" fn quotearg_n_style_colon(
    n: libc::c_int,
    s: libc::c_int,
    arg: *const libc::c_char,
) -> *mut libc::c_char {
    quotearg_n_style_colon_rust(n, s, arg)
}

fn fdadvise_rust(fd: c_int, offset: libc::off_t, len: libc::off_t, advice: c_int) {
    #[cfg(target_os = "linux")]
    {
        // Call posix_fadvise and ignore the return value
        let _ = unsafe { libc::posix_fadvise(fd, offset, len, advice) };
    }
}
#[no_mangle]
pub extern "C" fn fdadvise(fd: c_int, offset: libc::off_t, len: libc::off_t, advice: c_int) {
    fdadvise_rust(fd, offset, len, advice);
}

fn rpl_copy_file_range_rust(
    infd: c_int,
    pinoff: Option<&mut libc_off_t>,
    outfd: c_int,
    poutoff: Option<&mut libc_off_t>,
    length: size_t,
    flags: c_uint,
) -> ssize_t {
    static INIT: Once = Once::new();
    static OK: AtomicI8 = AtomicI8::new(0);

    INIT.call_once(|| {
        let mut name = libc_utsname {
            sysname: [0; 65],
            nodename: [0; 65],
            release: [0; 65],
            version: [0; 65],
            machine: [0; 65],
            domainname: [0; 65],
        };

        if unsafe { libc_uname(&mut name) } == 0 {
            let release = unsafe { std::ffi::CStr::from_ptr(name.release.as_ptr()) }
                .to_string_lossy()
                .into_owned();
            let mut parts = release.split('.').map(|s| s.parse::<u32>().unwrap_or(0));
            let major = parts.next().unwrap_or(0);
            let minor = parts.next().unwrap_or(0);

            if major > 5 || (major == 5 && minor > 2) {
                OK.store(1, AtomicOrdering::Relaxed);
            } else {
                OK.store(-1, AtomicOrdering::Relaxed);
            }
        } else {
            OK.store(-1, AtomicOrdering::Relaxed);
        }
    });

    if OK.load(AtomicOrdering::Relaxed) > 0 {
        // Safe because we are directly calling the libc function with valid arguments.
        let result = unsafe {
            libc::copy_file_range(
                infd,
                pinoff.map_or(std::ptr::null_mut(), |p| p as *mut libc_off_t),
                outfd,
                poutoff.map_or(std::ptr::null_mut(), |p| p as *mut libc_off_t),
                length,
                flags,
            )
        };

        if result >= 0 {
            return result;
        }
    }

    // If we reach here, either the kernel version is too old or the syscall failed.
    unsafe {
        *libc::__errno_location() = LIBC_ENOSYS;
    }
    -1
}
#[no_mangle]
pub extern "C" fn rpl_copy_file_range(
    infd: c_int,
    pinoff: *mut libc_off_t,
    outfd: c_int,
    poutoff: *mut libc_off_t,
    length: size_t,
    flags: c_uint,
) -> ssize_t {
    let pinoff = if pinoff.is_null() {
        None
    } else {
        // Safe because the pointer is provided by the caller and assumed valid.
        Some(unsafe { &mut *pinoff })
    };

    let poutoff = if poutoff.is_null() {
        None
    } else {
        // Safe because the pointer is provided by the caller and assumed valid.
        Some(unsafe { &mut *poutoff })
    };

    rpl_copy_file_range_rust(infd, pinoff, outfd, poutoff, length, flags)
}

fn is_ENOTSUP_rust(err: c_int) -> bool {
    err == LIBC_EOPNOTSUPP || (LIBC_ENOTSUP != LIBC_EOPNOTSUPP && err == LIBC_ENOTSUP)
}
#[no_mangle]
pub extern "C" fn is_ENOTSUP(err: libc::c_int) -> bool {
    is_ENOTSUP_rust(err)
}

fn xalignalloc_rust(alignment: libc::size_t, size: libc::size_t) -> *mut libc::c_void {
    // Ensure alignment and size are valid
    if alignment == 0 || size == 0 || !alignment.is_power_of_two() {
        unsafe { xalloc_die() };
    }

    // Create a layout with the specified alignment and size
    let layout = Layout::from_size_align(size, alignment);
    if let Ok(layout) = layout {
        // Allocate memory using the layout
        unsafe {
            let ptr = alloc(layout);
            if !ptr.is_null() {
                return ptr as *mut libc::c_void;
            }
        }
    }

    // If allocation fails, call xalloc_die
    unsafe { xalloc_die() };

    // This line will never be reached because xalloc_die will terminate the program
    std::ptr::null_mut()
}
#[no_mangle]
pub extern "C" fn xalignalloc(alignment: libc::size_t, size: libc::size_t) -> *mut libc::c_void {
    xalignalloc_rust(alignment, size)
}

fn safe_read_rust(fd: libc::c_int, buf: &mut [u8], mut count: usize) -> libc::ssize_t {
    const SYS_BUFSIZE_MAX: usize = 8192; // Replace with the actual value if defined elsewhere

    loop {
        // Attempt to read from the file descriptor
        let result = unsafe { libc::read(fd, buf.as_mut_ptr() as *mut libc::c_void, count) };

        if result >= 0 {
            // Successful read, return the number of bytes read
            return result;
        } else {
            // Handle errors
            let errno = unsafe { *libc::__errno_location() };

            if errno == libc::EINTR {
                // Interrupted by a signal, retry the operation
                continue;
            } else if errno == libc::EINVAL && count > SYS_BUFSIZE_MAX {
                // Adjust the count if it's larger than the system buffer size
                count = SYS_BUFSIZE_MAX;
            } else {
                // For other errors, return the result as is (negative value)
                return result;
            }
        }
    }
}

fn safe_write_rust(fd: libc::c_int, buf: &[u8], mut count: usize) -> libc::ssize_t {
    const SYS_BUFSIZE_MAX: usize = 8192; // Replace with the actual value if defined elsewhere

    loop {
        // Attempt to write to the file descriptor
        let result = unsafe { libc::write(fd, buf.as_ptr() as *const libc::c_void, count) };

        if result >= 0 {
            // Successful write, return the number of bytes written
            return result;
        } else {
            // Handle errors
            let errno = unsafe { *libc::__errno_location() };

            if errno == libc::EINTR {
                // Interrupted by a signal, retry the operation
                continue;
            } else if errno == libc::EINVAL && count > SYS_BUFSIZE_MAX {
                // Adjust the count if it's larger than the system buffer size
                count = SYS_BUFSIZE_MAX;
            } else {
                // For other errors, return the result as is (negative value)
                return result;
            }
        }
    }
}
#[no_mangle]
pub extern "C" fn safe_read(
    fd: libc::c_int,
    buf: *mut libc::c_void,
    count: libc::size_t,
) -> libc::ssize_t {
    // Safety: Ensure the buffer pointer is valid and not null
    if buf.is_null() {
        return -1;
    }

    // Convert the raw pointer to a mutable slice
    let buf_slice = unsafe { slice::from_raw_parts_mut(buf as *mut u8, count) };

    // Call the safe Rust function
    safe_read_rust(fd, buf_slice, count)
}

#[no_mangle]
pub extern "C" fn safe_write(
    fd: libc::c_int,
    buf: *const libc::c_void,
    count: libc::size_t,
) -> libc::ssize_t {
    // Safety: Ensure the buffer pointer is valid and not null
    if buf.is_null() {
        return -1;
    }

    // Convert the raw pointer to a slice
    let buf_slice = unsafe { slice::from_raw_parts(buf as *const u8, count) };

    // Call the safe Rust function
    safe_write_rust(fd, buf_slice, count)
}

fn fpurge_rust(fp: *mut libc::FILE) -> c_int {
    if fp.is_null() {
        return -1; // Return an error if the FILE pointer is null
    }

    unsafe {
        #[cfg(any(
            target_os = "linux",
            target_os = "android",
            target_os = "haiku",
            target_os = "solaris",
            target_os = "cygwin"
        ))]
        {
            // Use __fpurge if available
            extern "C" {
                fn __fpurge(fp: *mut libc::FILE);
            }
            __fpurge(fp);
            return 0;
        }

        #[cfg(any(
            target_os = "freebsd",
            target_os = "netbsd",
            target_os = "openbsd",
            target_os = "dragonfly",
            target_os = "macos"
        ))]
        {
            // Use fpurge if available
            extern "C" {
                fn fpurge(fp: *mut libc::FILE) -> c_int;
            }
            let result = fpurge(fp);
            if result == 0 {
                // Correct invariants if necessary
                let flags = (*fp)._flags;
                if (flags & libc::__SRD) != 0 {
                    (*fp)._w = 0;
                }
            }
            return result;
        }

        #[cfg(any(
            target_os = "gnu",
            target_os = "beos",
            target_os = "uclibc",
            target_os = "qnx",
            target_os = "mint",
            target_os = "plan9"
        ))]
        {
            // Handle other platforms with custom logic
            (*fp)._IO_read_end = (*fp)._IO_read_ptr;
            (*fp)._IO_write_ptr = (*fp)._IO_write_base;

            if !(*fp)._IO_save_base.is_null() {
                libc_free((*fp)._IO_save_base as *mut libc::c_void);
                (*fp)._IO_save_base = ptr::null_mut();
            }
            return 0;
        }

        #[cfg(not(any(
            target_os = "linux",
            target_os = "android",
            target_os = "haiku",
            target_os = "solaris",
            target_os = "cygwin",
            target_os = "freebsd",
            target_os = "netbsd",
            target_os = "openbsd",
            target_os = "dragonfly",
            target_os = "macos",
            target_os = "gnu",
            target_os = "beos",
            target_os = "uclibc",
            target_os = "qnx",
            target_os = "mint",
            target_os = "plan9"
        )))]
        {
            // Unsupported platform
            return -1; // Indicate failure
        }
    }
}
#[no_mangle]
pub extern "C" fn fpurge(fp: *mut libc::FILE) -> c_int {
    fpurge_rust(fp)
}

fn alignfree_rust(ptr: Option<NonNull<c_void>>) {
    if let Some(non_null_ptr) = ptr {
        // Convert NonNull<c_void> back to a raw pointer and free it using libc_free
        unsafe {
            libc_free(non_null_ptr.as_ptr());
        }
    }
}
#[no_mangle]
pub extern "C" fn alignfree(ptr: *mut c_void) {
    // Convert the raw pointer to an Option<NonNull<c_void>> for safe handling
    let non_null_ptr = NonNull::new(ptr);
    alignfree_rust(non_null_ptr);
}
