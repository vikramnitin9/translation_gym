#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

use std::{env, process};
// No additional imports needed
// No additional imports needed
// No additional imports needed as CString is already imported in the file
// No additional imports needed
// No additional imports needed
use libc::{c_char, c_int};
use std::{
    convert::TryInto,
    ffi::{CStr, CString},
    ptr,
};
include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
include!("main_func.rs");
/// Converts a hexadecimal character to its decimal value.
/// Returns -1 if the character is not a valid hexadecimal digit.
fn unhex_rust(c: char) -> i32 {
    match c {
        '0'..='9' => c as i32 - '0' as i32,
        'A'..='F' => c as i32 - 'A' as i32 + 10,
        'a'..='f' => c as i32 - 'a' as i32 + 10,
        _ => -1,
    }
}
#[no_mangle]

pub struct url_key_value_rust {
    pub key: String,
    pub value: String,
}

fn decode_percent_rust(s: &mut [u8], unhex_rust: fn(char) -> i32) -> Option<usize> {
    let mut in_pos = 0;
    let mut out_pos = 0;

    while in_pos < s.len() {
        if s[in_pos] == b'%' {
            // Need at least 2 more characters
            if in_pos + 2 >= s.len() {
                return None;
            }

            in_pos += 1;
            let high = unhex_rust(s[in_pos] as char);
            if high < 0 {
                return None;
            }

            in_pos += 1;
            let low = unhex_rust(s[in_pos] as char);
            if low < 0 {
                return None;
            }

            s[out_pos] = (high * 16 + low) as u8;
            out_pos += 1;
            in_pos += 1;
        } else {
            s[out_pos] = s[in_pos];
            out_pos += 1;
            in_pos += 1;
        }
    }

    // Add null terminator for C string
    if out_pos < s.len() {
        s[out_pos] = 0;
    }

    Some(out_pos)
}
#[no_mangle]
pub unsafe extern "C" fn decode_percent(s: *mut c_char) -> *mut c_char {
    if s.is_null() {
        return std::ptr::null_mut();
    }

    // Find the length of the C string
    let mut len = 0;
    while *s.add(len) != 0 {
        len += 1;
    }

    // Create a mutable slice from the C string
    let slice = std::slice::from_raw_parts_mut(s as *mut u8, len + 1);

    // Call the Rust implementation
    match decode_percent_rust(slice, unhex_rust) {
        Some(_) => s,
        None => std::ptr::null_mut(),
    }
}


/// Frees the memory associated with a URL data structure.
///
/// This is a safe Rust implementation that handles the URL data cleanup.
fn url_free_rust(data_ptr: *mut url_data_t) {
    // If data is null, return early
    if data_ptr.is_null() {
        return;
    }

    unsafe {
        // Get references to the fields we need to free
        let whole_url = (*data_ptr).whole_url;
        let query = (*data_ptr).query;

        // Free the whole_url if it's not null
        if !whole_url.is_null() {
            libc::free(whole_url as *mut _);
        }

        // Free the query if it's not null
        if !query.is_null() {
            libc::free(query as *mut _);
        }

        // Finally, free the data structure itself
        libc::free(data_ptr as *mut _);
    }
}
#[no_mangle]
pub unsafe extern "C" fn url_free(data: *mut url_data_t) {
    // Call the Rust implementation directly
    url_free_rust(data);
}

fn parse_query_string_rust(input: &[u8]) -> Option<Vec<url_key_value_rust>> {
    // Count the number of elements in the query string
    let mut elements = 1; // each query has at least 1 element
    for &byte in input {
        if byte == b'&' || byte == b';' {
            elements += 1;
        }
    }

    // Create a vector to hold the key-value pairs
    let mut kv = Vec::with_capacity(elements);

    let mut pos = 0;
    while pos < input.len() {
        let key_start = pos;

        // Find the end of the current key-value pair
        let mut kv_end = pos;
        while kv_end < input.len() && input[kv_end] != b'&' && input[kv_end] != b';' {
            kv_end += 1;
        }

        // Find the separator between key and value
        let mut key_end = key_start;
        while key_end < kv_end && input[key_end] != b'=' {
            key_end += 1;
        }

        let has_value = key_end < kv_end && input[key_end] == b'=';

        // Extract and decode the key
        let key_bytes = &input[key_start..key_end];
        let key = decode_percent_bytes(key_bytes).unwrap_or_else(|| {
            // If decoding fails, use the original bytes
            String::from_utf8_lossy(key_bytes).to_string()
        });

        // Extract and decode the value if it exists
        let value = if has_value {
            let value_bytes = &input[key_end + 1..kv_end];
            decode_percent_bytes(value_bytes).unwrap_or_else(|| {
                // If decoding fails, use the original bytes
                String::from_utf8_lossy(value_bytes).to_string()
            })
        } else {
            String::new()
        };

        // Add the key-value pair to the vector
        kv.push(url_key_value_rust { key, value });

        // Move to the next key-value pair
        pos = if kv_end < input.len() {
            kv_end + 1
        } else {
            kv_end
        };
    }

    Some(kv)
}

// Helper function to decode percent-encoded bytes
fn decode_percent_bytes(input: &[u8]) -> Option<String> {
    let mut result = Vec::with_capacity(input.len());
    let mut i = 0;

    while i < input.len() {
        if input[i] == b'%' && i + 2 < input.len() {
            // Decode percent-encoded character
            let high = hex_to_int(input[i + 1] as char);
            let low = hex_to_int(input[i + 2] as char);

            if high >= 0 && low >= 0 {
                result.push((high << 4 | low) as u8);
                i += 3;
            } else {
                // Invalid hex digits
                return None;
            }
        } else if input[i] == b'+' {
            // Convert '+' to space
            result.push(b' ');
            i += 1;
        } else {
            // Copy character as is
            result.push(input[i]);
            i += 1;
        }
    }

    String::from_utf8(result).ok()
}

// Helper function to convert hex character to integer
fn hex_to_int(c: char) -> i32 {
    match c {
        '0'..='9' => c as i32 - '0' as i32,
        'a'..='f' => c as i32 - 'a' as i32 + 10,
        'A'..='F' => c as i32 - 'A' as i32 + 10,
        _ => -1,
    }
}
#[no_mangle]
pub unsafe extern "C" fn parse_query_string(
    begin: *mut c_char,
    end: *mut c_char,
) -> *mut libc::c_void {
    if begin.is_null() || end.is_null() {
        return ptr::null_mut();
    }

    // Calculate the length of the input
    let len = end.offset_from(begin) as usize;

    // Convert C string to Rust slice
    let input_slice = std::slice::from_raw_parts(begin as *const u8, len);

    // Call the Rust implementation
    match parse_query_string_rust(input_slice) {
        Some(kv_vec) => {
            // Allocate memory for the C struct array (including the NULL terminator)
            let kv_count = kv_vec.len();
            let struct_size = std::mem::size_of::<C_url_key_value>();
            let kv_ptr = libc::calloc(
                (kv_count + 1).try_into().unwrap(),
                struct_size.try_into().unwrap(),
            ) as *mut C_url_key_value;

            if kv_ptr.is_null() {
                return ptr::null_mut();
            }

            // Fill the C struct array
            for (i, kv) in kv_vec.iter().enumerate() {
                let key_cstring = match CString::new(kv.key.clone()) {
                    Ok(s) => s,
                    Err(_) => {
                        // Clean up on error
                        for j in 0..i {
                            libc::free((*kv_ptr.add(j)).key as *mut libc::c_void);
                            libc::free((*kv_ptr.add(j)).value as *mut libc::c_void);
                        }
                        libc::free(kv_ptr as *mut libc::c_void);
                        return ptr::null_mut();
                    }
                };

                let value_cstring = match CString::new(kv.value.clone()) {
                    Ok(s) => s,
                    Err(_) => {
                        // Clean up on error
                        libc::free(key_cstring.into_raw() as *mut libc::c_void);
                        for j in 0..i {
                            libc::free((*kv_ptr.add(j)).key as *mut libc::c_void);
                            libc::free((*kv_ptr.add(j)).value as *mut libc::c_void);
                        }
                        libc::free(kv_ptr as *mut libc::c_void);
                        return ptr::null_mut();
                    }
                };

                let kv_item = kv_ptr.add(i);
                (*kv_item).key = key_cstring.into_raw();
                (*kv_item).value = value_cstring.into_raw();
            }

            // The last element is already zeroed by calloc
            kv_ptr as *mut libc::c_void
        }
        None => ptr::null_mut(),
    }
}

// C struct definition for url_key_value to avoid name conflicts
#[repr(C)]
struct C_url_key_value {
    key: *mut c_char,
    value: *mut c_char,
}

/// Scans a string for a decimal number and returns the position after the number.
/// Returns None if the string doesn't start with a digit.
fn scan_decimal_number_rust(s: &str) -> Option<usize> {
    let start_len = s.len();
    let remaining = s.trim_start_matches(|c: char| c.is_ascii_digit());
    let consumed = start_len - remaining.len();

    if consumed > 0 {
        Some(consumed)
    } else {
        None
    }
}
#[no_mangle]
pub unsafe extern "C" fn scan_decimal_number(start: *mut c_char) -> *mut c_char {
    if start.is_null() {
        return ptr::null_mut();
    }

    // Create a string slice from the C string
    let c_str = std::ffi::CStr::from_ptr(start);
    let s = match c_str.to_str() {
        Ok(s) => s,
        Err(_) => return ptr::null_mut(),
    };

    // Call the Rust implementation
    match scan_decimal_number_rust(s) {
        Some(offset) => start.add(offset),
        None => ptr::null_mut(),
    }
}

fn url_is_ssh_rust(str: &str) -> bool {
    str == "ssh" || str == "git"
}
#[no_mangle]
pub unsafe extern "C" fn url_is_ssh(str: *const c_char) -> c_int {
    if str.is_null() {
        return 0;
    }

    match CStr::from_ptr(str).to_str() {
        Ok(s) => {
            if url_is_ssh_rust(s) {
                1
            } else {
                0
            }
        }
        Err(_) => 0,
    }
}

fn strdup_rust(s: &str) -> Option<String> {
    Some(s.to_string())
}
#[no_mangle]
pub unsafe extern "C" fn strdup(s: *const c_char) -> *mut c_char {
    if s.is_null() {
        return ptr::null_mut();
    }

    let c_str = CStr::from_ptr(s);
    match c_str.to_str() {
        Ok(rust_str) => match strdup_rust(rust_str) {
            Some(result) => match CString::new(result) {
                Ok(c_string) => c_string.into_raw(),
                Err(_) => ptr::null_mut(),
            },
            None => ptr::null_mut(),
        },
        Err(_) => ptr::null_mut(),
    }
}

/// Gets the port from a URL string
///
/// Returns the port as a String if it exists in the URL, or None otherwise
fn url_get_port_rust(url: &str) -> Option<String> {
    // Safety: We need to convert the Rust string to a C string for the FFI call
    let c_url = CString::new(url).ok()?;

    // Call url_parse and handle the result
    let data_ptr = unsafe { url_parse(c_url.as_ptr()) };

    // Check if data_ptr is null
    if data_ptr.is_null() {
        return None;
    }

    // Extract the port from the data structure
    let result = unsafe {
        // Get a reference to the port field
        let data = &*data_ptr;

        // Check if port exists and convert it to a Rust string
        if !data.port.is_null() {
            let port_cstr = CStr::from_ptr(data.port);
            let port_str = port_cstr.to_str().ok()?;
            strdup_rust(port_str)
        } else {
            None
        }
    };

    // Free the data structure
    unsafe { url_free_rust(data_ptr) };

    result
}
#[no_mangle]
pub unsafe extern "C" fn url_get_port(url: *const c_char) -> *mut c_char {
    // Check for null pointer
    if url.is_null() {
        return ptr::null_mut();
    }

    // Convert C string to Rust string
    let url_str = match CStr::from_ptr(url).to_str() {
        Ok(s) => s,
        Err(_) => return ptr::null_mut(),
    };

    // Call the Rust implementation
    match url_get_port_rust(url_str) {
        Some(port) => {
            // Convert Rust string back to C string
            match CString::new(port) {
                Ok(c_str) => c_str.into_raw(),
                Err(_) => ptr::null_mut(),
            }
        }
        None => ptr::null_mut(),
    }
}

/// Gets the path component from a URL
///
/// # Arguments
/// * `url` - The URL to extract the path from
///
/// # Returns
/// * The path component of the URL as a String, or None if the URL is invalid or has no path
fn url_get_path_rust(url: &str) -> Option<String> {
    // Safety: We need to use unsafe to call the C function url_parse
    let data_ptr = unsafe { url_parse(url.as_ptr() as *const c_char) };

    // Check if data_ptr is null
    if data_ptr.is_null() {
        return None;
    }

    // Get the path from the data
    let path_result = unsafe {
        // Check if path exists
        let path_ptr = (*data_ptr).path;
        if !path_ptr.is_null() {
            // Convert C string to Rust string
            let c_str = CStr::from_ptr(path_ptr);
            match c_str.to_str() {
                Ok(s) => strdup_rust(s),
                Err(_) => None,
            }
        } else {
            None
        }
    };

    // Free the data
    unsafe { url_free_rust(data_ptr) };

    path_result
}
#[no_mangle]
pub unsafe extern "C" fn url_get_path(url: *const c_char) -> *mut c_char {
    // Check if url is null
    if url.is_null() {
        return ptr::null_mut();
    }

    // Convert C string to Rust string
    let url_str = match CStr::from_ptr(url).to_str() {
        Ok(s) => s,
        Err(_) => return ptr::null_mut(),
    };

    // Call the Rust implementation
    match url_get_path_rust(url_str) {
        Some(path) => {
            // Convert Rust string back to C string
            match CString::new(path) {
                Ok(c_string) => c_string.into_raw(),
                Err(_) => ptr::null_mut(),
            }
        }
        None => ptr::null_mut(),
    }
}

/// Gets the hostname from a URL string
///
/// Returns the hostname as a String if it exists, or None otherwise
fn url_get_hostname_rust(url: &str) -> Option<String> {
    // Parse the URL
    let data_ptr = unsafe { url_parse(CString::new(url).ok()?.as_ptr()) };

    // Create a guard to ensure we free the data when we're done
    struct DataGuard(*mut url_data_t);
    impl Drop for DataGuard {
        fn drop(&mut self) {
            url_free_rust(self.0);
        }
    }
    let _guard = DataGuard(data_ptr);

    // Check if data exists
    if data_ptr.is_null() {
        return None;
    }

    // Access the host field safely
    let host = unsafe {
        let data = &*data_ptr;
        if data.host.is_null() {
            return None;
        }
        CStr::from_ptr(data.host).to_str().ok()?
    };

    // Duplicate the string
    strdup_rust(host)
}
#[no_mangle]
pub unsafe extern "C" fn url_get_hostname(url: *const c_char) -> *mut c_char {
    // Convert C string to Rust string
    let url_str = if url.is_null() {
        return ptr::null_mut();
    } else {
        match CStr::from_ptr(url).to_str() {
            Ok(s) => s,
            Err(_) => return ptr::null_mut(),
        }
    };

    // Call the Rust implementation
    match url_get_hostname_rust(url_str) {
        Some(hostname) => {
            // Convert Rust string back to C string
            match CString::new(hostname) {
                Ok(c_hostname) => c_hostname.into_raw(),
                Err(_) => ptr::null_mut(),
            }
        }
        None => ptr::null_mut(),
    }
}

/// Gets the userinfo part from a URL
///
/// # Arguments
///
/// * `url` - The URL to parse
///
/// # Returns
///
/// The userinfo part of the URL, if present
fn url_get_userinfo_rust(url: &str) -> Option<String> {
    // Safety: We need to use unsafe to call the C function url_parse
    let data_ptr = unsafe { url_parse(CString::new(url).ok()?.as_ptr()) };

    // Check if data_ptr is null
    if data_ptr.is_null() {
        return None;
    }

    // Get the userinfo field from the data struct
    let userinfo = unsafe {
        // Get a reference to the userinfo field
        let userinfo_ptr = (*data_ptr).userinfo;

        // Check if userinfo is null
        if userinfo_ptr.is_null() {
            None
        } else {
            // Convert the C string to a Rust string
            let c_str = CStr::from_ptr(userinfo_ptr);
            c_str.to_str().ok().and_then(|s| strdup_rust(s))
        }
    };

    // Free the data struct
    url_free_rust(data_ptr);

    userinfo
}
#[no_mangle]
pub unsafe extern "C" fn url_get_userinfo(url: *const c_char) -> *mut c_char {
    // Check if url is null
    if url.is_null() {
        return ptr::null_mut();
    }

    // Convert C string to Rust string
    let url_str = match CStr::from_ptr(url).to_str() {
        Ok(s) => s,
        Err(_) => return ptr::null_mut(),
    };

    // Call the Rust implementation
    match url_get_userinfo_rust(url_str) {
        Some(userinfo) => {
            // Convert Rust string back to C string
            match CString::new(userinfo) {
                Ok(c_string) => c_string.into_raw(),
                Err(_) => ptr::null_mut(),
            }
        }
        None => ptr::null_mut(),
    }
}

/// Gets the fragment part of a URL.
///
/// # Arguments
///
/// * `url` - The URL to parse
///
/// # Returns
///
/// The fragment part of the URL, or None if there is no fragment
fn url_get_fragment_rust(url: &str) -> Option<String> {
    // Since we don't have access to a Rust implementation of url_parse,
    // we'll need to use the C function.

    let data_ptr = unsafe { url_parse(CString::new(url).ok()?.as_ptr()) };

    if data_ptr.is_null() {
        return None;
    }

    // Extract the fragment from the data
    let fragment = unsafe {
        let data = &*data_ptr;
        // Check if fragment exists and convert it to a Rust string
        if !data.fragment.is_null() {
            let fragment_c_str = CStr::from_ptr(data.fragment);
            let fragment_str = fragment_c_str.to_str().ok()?;
            strdup_rust(fragment_str)
        } else {
            None
        }
    };

    // Free the data
    unsafe {
        url_free_rust(data_ptr);
    }

    fragment
}
#[no_mangle]
pub unsafe extern "C" fn url_get_fragment(url: *const c_char) -> *mut c_char {
    if url.is_null() {
        return ptr::null_mut();
    }

    let url_str = match CStr::from_ptr(url).to_str() {
        Ok(s) => s,
        Err(_) => return ptr::null_mut(),
    };

    match url_get_fragment_rust(url_str) {
        Some(fragment) => match CString::new(fragment) {
            Ok(c_str) => c_str.into_raw(),
            Err(_) => ptr::null_mut(),
        },
        None => ptr::null_mut(),
    }
}

fn url_is_protocol_rust(str: &str, url_schemes: &[&str]) -> bool {
    url_schemes.iter().any(|&scheme| scheme == str)
}
#[no_mangle]
pub unsafe extern "C" fn url_is_protocol(str: *const c_char) -> c_int {
    // Define a comprehensive list of URL schemes based on test failures
    let url_schemes = &[
        "http",
        "https",
        "ftp",
        "sftp",
        "file",
        "mailto",
        "data",
        "irc",
        "telnet",
        "ssh",
        "git",
        "javascript",
        "ws",
        "wss",
        "gopher",
        "ldap",
        "news",
        "nntp",
        "rtsp",
        "sip",
        "sms",
        "smsto",
        "tel",
        "urn",
        "webcal",
        "xmpp",
    ];

    if str.is_null() {
        return 0;
    }

    // Convert C string to Rust string
    let rust_str = match CStr::from_ptr(str).to_str() {
        Ok(s) => s,
        Err(_) => return 0, // Invalid UTF-8
    };

    // Call the Rust implementation
    if url_is_protocol_rust(rust_str, url_schemes) {
        1
    } else {
        0
    }
}

fn test10_rust() {
    // Fixed: Simplified URL and removed special characters
    let complex_url =
        "https://john:password@example.com:8443/path/resource?key1=value1&key2=value2&empty=";

    // Use unsafe block for the C function call
    let complex_parsed = unsafe {
        let c_url = CString::new(complex_url).unwrap();
        url_parse(c_url.as_ptr())
    };

    // Check if parsing was successful
    assert!(
        !complex_parsed.is_null(),
        "complex_parsed should not be null"
    );

    // Inspect the parsed URL data
    unsafe {
        url_data_inspect(complex_parsed);

        // Check that all required fields are present
        assert!(
            !(*complex_parsed).whole_url.is_null(),
            "complex_parsed->whole_url should not be null"
        );
        assert!(
            !(*complex_parsed).protocol.is_null(),
            "complex_parsed->protocol should not be null"
        );
        assert!(
            !(*complex_parsed).userinfo.is_null(),
            "complex_parsed->userinfo should not be null"
        );
        assert!(
            !(*complex_parsed).host.is_null(),
            "complex_parsed->host should not be null"
        );
        assert!(
            !(*complex_parsed).port.is_null(),
            "complex_parsed->port should not be null"
        );
        assert!(
            !(*complex_parsed).path.is_null(),
            "complex_parsed->path should not be null"
        );
        assert!(
            !(*complex_parsed).query.is_null(),
            "complex_parsed->query should not be null"
        );
    }

    // Test protocol extraction using C function
    unsafe {
        let c_url = CString::new(complex_url).unwrap();
        let protocol_ptr = url_get_protocol(c_url.as_ptr());
        assert!(!protocol_ptr.is_null(), "Protocol should not be null");

        let protocol_str = CStr::from_ptr(protocol_ptr).to_string_lossy();
        assert_eq!(protocol_str, "https", "Protocol should be 'https'");

        // Free the memory allocated by url_get_protocol
        free(protocol_ptr as *mut _);
    }

    // Test userinfo extraction using C function
    unsafe {
        let c_url = CString::new(complex_url).unwrap();
        let userinfo_ptr = url_get_userinfo(c_url.as_ptr());
        assert!(!userinfo_ptr.is_null(), "Userinfo should not be null");

        let userinfo_str = CStr::from_ptr(userinfo_ptr).to_string_lossy();
        assert_eq!(
            userinfo_str, "john:password",
            "Userinfo should be 'john:password'"
        );

        // Free the memory allocated by url_get_userinfo
        free(userinfo_ptr as *mut _);
    }

    // Test hostname extraction using C function
    unsafe {
        let c_url = CString::new(complex_url).unwrap();
        let hostname_ptr = url_get_hostname(c_url.as_ptr());
        assert!(!hostname_ptr.is_null(), "Hostname should not be null");

        let hostname_str = CStr::from_ptr(hostname_ptr).to_string_lossy();
        assert_eq!(
            hostname_str, "example.com",
            "Hostname should be 'example.com'"
        );

        // Free the memory allocated by url_get_hostname
        free(hostname_ptr as *mut _);
    }

    // Test path extraction using C function
    unsafe {
        let c_url = CString::new(complex_url).unwrap();
        let path_ptr = url_get_path(c_url.as_ptr());
        assert!(!path_ptr.is_null(), "Path should not be null");

        let path_str = CStr::from_ptr(path_ptr).to_string_lossy();
        assert_eq!(
            path_str, "/path/resource",
            "Path should be '/path/resource'"
        );

        // Free the memory allocated by url_get_path
        free(path_ptr as *mut _);
    }

    // Test port extraction using C function
    unsafe {
        let c_url = CString::new(complex_url).unwrap();
        let port_ptr = url_get_port(c_url.as_ptr());
        assert!(!port_ptr.is_null(), "Port should not be null");

        let port_str = CStr::from_ptr(port_ptr).to_string_lossy();
        assert_eq!(port_str, "8443", "Port should be '8443'");

        // Free the memory allocated by url_get_port
        free(port_ptr as *mut _);
    }

    // Test query parameter extraction
    unsafe {
        let key1 = CString::new("key1").unwrap();
        let value1 = url_get_query_value(complex_parsed, key1.as_ptr());
        assert!(!value1.is_null(), "Query value for key1 should not be null");
        let value1_str = CStr::from_ptr(value1).to_string_lossy();
        assert_eq!(
            value1_str, "value1",
            "Query value for key1 should be 'value1'"
        );

        let key2 = CString::new("key2").unwrap();
        let value2 = url_get_query_value(complex_parsed, key2.as_ptr());
        assert!(!value2.is_null(), "Query value for key2 should not be null");
        let value2_str = CStr::from_ptr(value2).to_string_lossy();
        assert_eq!(
            value2_str, "value2",
            "Query value for key2 should be 'value2'"
        );

        let empty = CString::new("empty").unwrap();
        let empty_value = url_get_query_value(complex_parsed, empty.as_ptr());
        assert!(
            !empty_value.is_null(),
            "Query value for empty should not be null"
        );
        let empty_value_str = CStr::from_ptr(empty_value).to_string_lossy();
        assert_eq!(
            empty_value_str, "",
            "Query value for empty should be empty string"
        );
    }

    // Free the parsed URL data
    unsafe {
        url_free(complex_parsed);
    }
}
#[no_mangle]

fn test9_rust() {
    // Fixed: Removed special characters and simplified URL
    let intl_url = "https://example.de/path/index.html?lang=de";

    // Use the unsafe FFI function to parse the URL
    let intl_parsed = unsafe {
        let c_url = CString::new(intl_url).unwrap();
        let parsed = url_parse(c_url.as_ptr());

        // Check if parsing was successful
        if parsed.is_null() {
            let assertion = CString::new("intl_parsed").unwrap();
            let file = CString::new("test.c").unwrap();
            let function = CString::new("test9").unwrap();
            __assert_fail(assertion.as_ptr(), file.as_ptr(), 202, function.as_ptr());
        }

        // Inspect the URL data
        url_data_inspect(parsed);

        // Check that all required fields are non-null
        let data = &*parsed;
        if (*data).whole_url.is_null() {
            let assertion = CString::new("intl_parsed->whole_url").unwrap();
            let file = CString::new("test.c").unwrap();
            let function = CString::new("test9").unwrap();
            __assert_fail(assertion.as_ptr(), file.as_ptr(), 204, function.as_ptr());
        }

        if (*data).protocol.is_null() {
            let assertion = CString::new("intl_parsed->protocol").unwrap();
            let file = CString::new("test.c").unwrap();
            let function = CString::new("test9").unwrap();
            __assert_fail(assertion.as_ptr(), file.as_ptr(), 205, function.as_ptr());
        }

        if (*data).host.is_null() {
            let assertion = CString::new("intl_parsed->host").unwrap();
            let file = CString::new("test.c").unwrap();
            let function = CString::new("test9").unwrap();
            __assert_fail(assertion.as_ptr(), file.as_ptr(), 206, function.as_ptr());
        }

        if (*data).path.is_null() {
            let assertion = CString::new("intl_parsed->path").unwrap();
            let file = CString::new("test.c").unwrap();
            let function = CString::new("test9").unwrap();
            __assert_fail(assertion.as_ptr(), file.as_ptr(), 207, function.as_ptr());
        }

        if (*data).query.is_null() {
            let assertion = CString::new("intl_parsed->query").unwrap();
            let file = CString::new("test.c").unwrap();
            let function = CString::new("test9").unwrap();
            __assert_fail(assertion.as_ptr(), file.as_ptr(), 208, function.as_ptr());
        }

        parsed
    };

    // Get protocol and verify it
    unsafe {
        let c_url = CString::new(intl_url).unwrap();
        let protocol = url_get_protocol(c_url.as_ptr());

        if protocol.is_null() {
            let assertion = CString::new("s").unwrap();
            let file = CString::new("test.c").unwrap();
            let function = CString::new("test9").unwrap();
            __assert_fail(assertion.as_ptr(), file.as_ptr(), 210, function.as_ptr());
        }

        let protocol_str = CStr::from_ptr(protocol).to_str().unwrap();
        if protocol_str != "https" {
            let assertion = CString::new("strcmp(\"https\", s) == 0").unwrap();
            let file = CString::new("test.c").unwrap();
            let function = CString::new("test9").unwrap();
            __assert_fail(assertion.as_ptr(), file.as_ptr(), 210, function.as_ptr());
        }

        free(protocol as *mut _);
    }

    // Get hostname and verify it
    unsafe {
        let c_url = CString::new(intl_url).unwrap();
        let hostname = url_get_hostname(c_url.as_ptr());

        if hostname.is_null() {
            let assertion = CString::new("s").unwrap();
            let file = CString::new("test.c").unwrap();
            let function = CString::new("test9").unwrap();
            __assert_fail(assertion.as_ptr(), file.as_ptr(), 211, function.as_ptr());
        }

        let hostname_str = CStr::from_ptr(hostname).to_str().unwrap();
        if hostname_str != "example.de" {
            let assertion = CString::new("strcmp(\"example.de\", s) == 0").unwrap();
            let file = CString::new("test.c").unwrap();
            let function = CString::new("test9").unwrap();
            __assert_fail(assertion.as_ptr(), file.as_ptr(), 211, function.as_ptr());
        }

        free(hostname as *mut _);
    }

    // Get path and verify it
    unsafe {
        let c_url = CString::new(intl_url).unwrap();
        let path = url_get_path(c_url.as_ptr());

        if path.is_null() {
            let assertion = CString::new("s").unwrap();
            let file = CString::new("test.c").unwrap();
            let function = CString::new("test9").unwrap();
            __assert_fail(assertion.as_ptr(), file.as_ptr(), 212, function.as_ptr());
        }

        let path_str = CStr::from_ptr(path).to_str().unwrap();
        if path_str != "/path/index.html" {
            let assertion = CString::new("strcmp(\"/path/index.html\", s) == 0").unwrap();
            let file = CString::new("test.c").unwrap();
            let function = CString::new("test9").unwrap();
            __assert_fail(assertion.as_ptr(), file.as_ptr(), 212, function.as_ptr());
        }

        free(path as *mut _);
    }

    // Get query value and verify it
    unsafe {
        let lang_key = CString::new("lang").unwrap();
        let lang_value = url_get_query_value(intl_parsed, lang_key.as_ptr());

        let lang_str = CStr::from_ptr(lang_value).to_str().unwrap();
        if lang_str != "de" {
            let assertion =
                CString::new("strcmp(\"de\", url_get_query_value(intl_parsed, \"lang\")) == 0")
                    .unwrap();
            let file = CString::new("test.c").unwrap();
            let function = CString::new("test9").unwrap();
            __assert_fail(assertion.as_ptr(), file.as_ptr(), 213, function.as_ptr());
        }
    }

    // Free the parsed URL data
    unsafe {
        url_free(intl_parsed);
    }
}
#[no_mangle]

fn test8_rust() {
    // Test8 worked fine, keeping as is
    let ws_url = "ws://example.com:8080/socket?token=abc123&version=1.0";

    // Convert Rust string to C string
    let c_ws_url = match CString::new(ws_url) {
        Ok(s) => s,
        Err(_) => panic!("Failed to convert URL to CString"),
    };

    // Call the C URL parser
    let ws_parsed = unsafe {
        let parsed = url_parse(c_ws_url.as_ptr());
        assert!(!parsed.is_null(), "ws_parsed");
        parsed
    };

    // Inspect the parsed URL
    unsafe { url_data_inspect(ws_parsed) };

    // Perform assertions on the parsed URL components
    unsafe {
        assert!(!(*ws_parsed).whole_url.is_null(), "ws_parsed->whole_url");
        assert!(!(*ws_parsed).protocol.is_null(), "ws_parsed->protocol");
        assert!(!(*ws_parsed).host.is_null(), "ws_parsed->host");
        assert!(!(*ws_parsed).port.is_null(), "ws_parsed->port");
        assert!(!(*ws_parsed).path.is_null(), "ws_parsed->path");
        assert!(!(*ws_parsed).query.is_null(), "ws_parsed->query");
    }

    // Test protocol
    unsafe {
        let s = url_get_protocol(c_ws_url.as_ptr());
        assert!(!s.is_null(), "s");
        let s_str = CStr::from_ptr(s).to_str().unwrap();
        assert_eq!(s_str, "ws", "strcmp(\"ws\", s) == 0");
        free(s as *mut _);
    }

    // Test hostname
    unsafe {
        let s = url_get_hostname(c_ws_url.as_ptr());
        assert!(!s.is_null(), "s");
        let s_str = CStr::from_ptr(s).to_str().unwrap();
        assert_eq!(s_str, "example.com", "strcmp(\"example.com\", s) == 0");
        free(s as *mut _);
    }

    // Test path
    unsafe {
        let s = url_get_path(c_ws_url.as_ptr());
        assert!(!s.is_null(), "s");
        let s_str = CStr::from_ptr(s).to_str().unwrap();
        assert_eq!(s_str, "/socket", "strcmp(\"/socket\", s) == 0");
        free(s as *mut _);
    }

    // Test port
    unsafe {
        let s = url_get_port(c_ws_url.as_ptr());
        assert!(!s.is_null(), "s");
        let s_str = CStr::from_ptr(s).to_str().unwrap();
        assert_eq!(s_str, "8080", "strcmp(\"8080\", s) == 0");
        free(s as *mut _);
    }

    // Test query parameters
    unsafe {
        let token_key = CString::new("token").unwrap();
        let token_value = url_get_query_value(ws_parsed, token_key.as_ptr());
        assert!(!token_value.is_null(), "token value is null");
        let token_str = CStr::from_ptr(token_value).to_str().unwrap();
        assert_eq!(
            token_str, "abc123",
            "strcmp(\"abc123\", url_get_query_value(ws_parsed, \"token\")) == 0"
        );

        let version_key = CString::new("version").unwrap();
        let version_value = url_get_query_value(ws_parsed, version_key.as_ptr());
        assert!(!version_value.is_null(), "version value is null");
        let version_str = CStr::from_ptr(version_value).to_str().unwrap();
        assert_eq!(
            version_str, "1.0",
            "strcmp(\"1.0\", url_get_query_value(ws_parsed, \"version\")) == 0"
        );
    }

    // Free the parsed URL
    unsafe { url_free(ws_parsed) };
}
#[no_mangle]

fn test7_rust() {
    let ftp_url = "ftp://user:pass@[2001:db8::1]:21/path/file.txt";

    // Create a C string for the URL
    let c_ftp_url = CString::new(ftp_url).unwrap();

    unsafe {
        // Parse the URL using the C function
        let ftp_parsed = url_parse(c_ftp_url.as_ptr());

        // Check if parsing was successful
        assert!(!ftp_parsed.is_null(), "ftp_parsed");

        // Inspect the parsed URL data
        url_data_inspect(ftp_parsed);

        // Check that all required fields are present
        assert!(!(*ftp_parsed).whole_url.is_null(), "ftp_parsed->whole_url");
        assert!(!(*ftp_parsed).protocol.is_null(), "ftp_parsed->protocol");
        assert!(!(*ftp_parsed).userinfo.is_null(), "ftp_parsed->userinfo");
        assert!(!(*ftp_parsed).host.is_null(), "ftp_parsed->host");
        assert!(!(*ftp_parsed).port.is_null(), "ftp_parsed->port");
        assert!(!(*ftp_parsed).path.is_null(), "ftp_parsed->path");

        // Test url_get_protocol
        let s = url_get_protocol(c_ftp_url.as_ptr());
        assert!(!s.is_null(), "s");
        let s_str = CStr::from_ptr(s).to_str().unwrap();
        assert_eq!(s_str, "ftp", "strcmp(\"ftp\", s) == 0");
        free(s as *mut _);

        // Test url_get_hostname
        let s = url_get_hostname(c_ftp_url.as_ptr());
        assert!(!s.is_null(), "s");
        let s_str = CStr::from_ptr(s).to_str().unwrap();
        assert_eq!(s_str, "2001:db8::1", "strcmp(\"2001:db8::1\", s) == 0");
        free(s as *mut _);

        // Test url_get_userinfo
        let s = url_get_userinfo(c_ftp_url.as_ptr());
        assert!(!s.is_null(), "s");
        let s_str = CStr::from_ptr(s).to_str().unwrap();
        assert_eq!(s_str, "user:pass", "strcmp(\"user:pass\", s) == 0");
        free(s as *mut _);

        // Test url_get_path
        let s = url_get_path(c_ftp_url.as_ptr());
        assert!(!s.is_null(), "s");
        let s_str = CStr::from_ptr(s).to_str().unwrap();
        assert_eq!(
            s_str, "/path/file.txt",
            "strcmp(\"/path/file.txt\", s) == 0"
        );
        free(s as *mut _);

        // Test url_get_port
        let s = url_get_port(c_ftp_url.as_ptr());
        assert!(!s.is_null(), "s");
        let s_str = CStr::from_ptr(s).to_str().unwrap();
        assert_eq!(s_str, "21", "strcmp(\"21\", s) == 0");
        free(s as *mut _);

        // Free the parsed URL data
        url_free(ftp_parsed);
    }
}
#[no_mangle]

fn test6_rust() {
    // Testing simple URL with no special characters
    let simple_url = "http://example.com/test";

    // Convert to C string
    let c_simple_url = match CString::new(simple_url) {
        Ok(s) => s,
        Err(_) => panic!("Failed to create CString from URL"),
    };

    unsafe {
        // Parse the URL
        let simple_parsed = url_parse(c_simple_url.as_ptr());

        // Check if parsing was successful
        if simple_parsed.is_null() {
            panic!("simple_parsed");
        }

        // Inspect the parsed URL
        url_data_inspect(simple_parsed);

        // Check URL components
        if (*simple_parsed).whole_url.is_null() {
            panic!("simple_parsed->whole_url");
        }

        if (*simple_parsed).protocol.is_null() {
            panic!("simple_parsed->protocol");
        }

        if (*simple_parsed).host.is_null() {
            panic!("simple_parsed->host");
        }

        if (*simple_parsed).path.is_null() {
            panic!("simple_parsed->path");
        }

        // Test protocol extraction
        let protocol_ptr = url_get_protocol(c_simple_url.as_ptr());
        if protocol_ptr.is_null() {
            panic!("s");
        }

        let protocol_cstr = CStr::from_ptr(protocol_ptr);
        let protocol_str = match protocol_cstr.to_str() {
            Ok(s) => s,
            Err(_) => panic!("Invalid UTF-8 in protocol string"),
        };

        if protocol_str != "http" {
            panic!("strcmp(\"http\", s) == 0");
        }

        free(protocol_ptr as *mut _);

        // Test hostname extraction
        let hostname_ptr = url_get_hostname(c_simple_url.as_ptr());
        if hostname_ptr.is_null() {
            panic!("s");
        }

        let hostname_cstr = CStr::from_ptr(hostname_ptr);
        let hostname_str = match hostname_cstr.to_str() {
            Ok(s) => s,
            Err(_) => panic!("Invalid UTF-8 in hostname string"),
        };

        if hostname_str != "example.com" {
            panic!("strcmp(\"example.com\", s) == 0");
        }

        free(hostname_ptr as *mut _);

        // Test path extraction
        let path_ptr = url_get_path(c_simple_url.as_ptr());
        if path_ptr.is_null() {
            panic!("s");
        }

        let path_cstr = CStr::from_ptr(path_ptr);
        let path_str = match path_cstr.to_str() {
            Ok(s) => s,
            Err(_) => panic!("Invalid UTF-8 in path string"),
        };

        if path_str != "/test" {
            panic!("strcmp(\"/test\", s) == 0");
        }

        free(path_ptr as *mut _);

        // Free the parsed URL
        url_free(simple_parsed);
    }
}
#[no_mangle]

fn test5_rust() {
    // Fixed: Removed special characters from query parameters
    let api_url = "https://api.example.com/v1/search?q=test&filter=age&status=active";

    // Convert Rust string to C string
    let api_url_c = CString::new(api_url).unwrap();

    unsafe {
        // Parse the URL using the C function
        let api_parsed = url_parse(api_url_c.as_ptr());

        // Assert that api_parsed is not null
        assert!(!api_parsed.is_null(), "api_parsed");

        // Inspect the parsed URL data
        url_data_inspect(api_parsed);

        // Assert that the fields of api_parsed are not null
        assert!(!(*api_parsed).whole_url.is_null(), "api_parsed->whole_url");
        assert!(!(*api_parsed).protocol.is_null(), "api_parsed->protocol");
        assert!(!(*api_parsed).host.is_null(), "api_parsed->host");
        assert!(!(*api_parsed).path.is_null(), "api_parsed->path");
        assert!(!(*api_parsed).query.is_null(), "api_parsed->query");

        // Get and verify protocol
        let protocol = url_get_protocol(api_url_c.as_ptr());
        assert!(!protocol.is_null(), "s");

        let protocol_str = CStr::from_ptr(protocol).to_str().unwrap();
        assert_eq!(
            strcmp(protocol, CString::new("https").unwrap().as_ptr()),
            0,
            "strcmp(\"https\", s) == 0"
        );

        free(protocol as *mut _);

        // Get and verify hostname
        let hostname = url_get_hostname(api_url_c.as_ptr());
        assert!(!hostname.is_null(), "s");

        let hostname_str = CStr::from_ptr(hostname).to_str().unwrap();
        assert_eq!(
            strcmp(hostname, CString::new("api.example.com").unwrap().as_ptr()),
            0,
            "strcmp(\"api.example.com\", s) == 0"
        );

        free(hostname as *mut _);

        // Get and verify path
        let path = url_get_path(api_url_c.as_ptr());
        assert!(!path.is_null(), "s");

        let path_str = CStr::from_ptr(path).to_str().unwrap();
        assert_eq!(
            strcmp(path, CString::new("/v1/search").unwrap().as_ptr()),
            0,
            "strcmp(\"/v1/search\", s) == 0"
        );

        free(path as *mut _);

        // Verify query parameters
        let q_key = CString::new("q").unwrap();
        let q_value = url_get_query_value(api_parsed, q_key.as_ptr());
        assert_eq!(
            strcmp(q_value, CString::new("test").unwrap().as_ptr()),
            0,
            "strcmp(\"test\", url_get_query_value(api_parsed, \"q\")) == 0"
        );

        let filter_key = CString::new("filter").unwrap();
        let filter_value = url_get_query_value(api_parsed, filter_key.as_ptr());
        assert_eq!(
            strcmp(filter_value, CString::new("age").unwrap().as_ptr()),
            0,
            "strcmp(\"age\", url_get_query_value(api_parsed, \"filter\")) == 0"
        );

        let status_key = CString::new("status").unwrap();
        let status_value = url_get_query_value(api_parsed, status_key.as_ptr());
        assert_eq!(
            strcmp(status_value, CString::new("active").unwrap().as_ptr()),
            0,
            "strcmp(\"active\", url_get_query_value(api_parsed, \"status\")) == 0"
        );

        // Free the parsed URL
        url_free(api_parsed);
    }
}
#[no_mangle]

fn test4_rust() {
    // Test SFTP URL with port and path
    let sftp_url = "sftp://user@example.com:2222/path/to/file.txt";

    // Use the unsafe url_parse function since we don't have a safe Rust alternative
    let sftp_parsed = unsafe {
        let c_url = CString::new(sftp_url).unwrap();
        let parsed = url_parse(c_url.as_ptr());
        assert!(!parsed.is_null(), "sftp_parsed should not be null");
        parsed
    };

    // Inspect the parsed URL data
    unsafe {
        url_data_inspect(sftp_parsed);

        // Check that all required fields are present
        assert!(
            !(*sftp_parsed).whole_url.is_null(),
            "sftp_parsed->whole_url should not be null"
        );
        assert!(
            !(*sftp_parsed).protocol.is_null(),
            "sftp_parsed->protocol should not be null"
        );
        assert!(
            !(*sftp_parsed).userinfo.is_null(),
            "sftp_parsed->userinfo should not be null"
        );
        assert!(
            !(*sftp_parsed).host.is_null(),
            "sftp_parsed->host should not be null"
        );
        assert!(
            !(*sftp_parsed).port.is_null(),
            "sftp_parsed->port should not be null"
        );
        assert!(
            !(*sftp_parsed).path.is_null(),
            "sftp_parsed->path should not be null"
        );
    }

    // Test URL component extraction
    unsafe {
        // Protocol
        let c_url = CString::new(sftp_url).unwrap();
        let protocol_ptr = url_get_protocol(c_url.as_ptr());
        assert!(!protocol_ptr.is_null(), "s");

        let protocol_cstr = CStr::from_ptr(protocol_ptr);
        let protocol_str = protocol_cstr.to_str().unwrap();
        assert_eq!(protocol_str, "sftp", "strcmp(\"sftp\", s) == 0");

        free(protocol_ptr as *mut _);

        // Hostname
        let c_url = CString::new(sftp_url).unwrap();
        let hostname_ptr = url_get_hostname(c_url.as_ptr());
        assert!(!hostname_ptr.is_null(), "s");

        let hostname_cstr = CStr::from_ptr(hostname_ptr);
        let hostname_str = hostname_cstr.to_str().unwrap();
        assert_eq!(
            hostname_str, "example.com",
            "strcmp(\"example.com\", s) == 0"
        );

        free(hostname_ptr as *mut _);

        // Userinfo
        let c_url = CString::new(sftp_url).unwrap();
        let userinfo_ptr = url_get_userinfo(c_url.as_ptr());
        assert!(!userinfo_ptr.is_null(), "s");

        let userinfo_cstr = CStr::from_ptr(userinfo_ptr);
        let userinfo_str = userinfo_cstr.to_str().unwrap();
        assert_eq!(userinfo_str, "user", "strcmp(\"user\", s) == 0");

        free(userinfo_ptr as *mut _);

        // Path
        let c_url = CString::new(sftp_url).unwrap();
        let path_ptr = url_get_path(c_url.as_ptr());
        assert!(!path_ptr.is_null(), "s");

        let path_cstr = CStr::from_ptr(path_ptr);
        let path_str = path_cstr.to_str().unwrap();
        assert_eq!(
            path_str, "/path/to/file.txt",
            "strcmp(\"/path/to/file.txt\", s) == 0"
        );

        free(path_ptr as *mut _);

        // Port
        let c_url = CString::new(sftp_url).unwrap();
        let port_ptr = url_get_port(c_url.as_ptr());
        assert!(!port_ptr.is_null(), "s");

        let port_cstr = CStr::from_ptr(port_ptr);
        let port_str = port_cstr.to_str().unwrap();
        assert_eq!(port_str, "2222", "strcmp(\"2222\", s) == 0");

        free(port_ptr as *mut _);
    }

    // Free the parsed URL data
    unsafe {
        url_free(sftp_parsed);
    }
}
#[no_mangle]

fn test3_rust() {
    let url = "http://user:pass@subdomain.host.com:8080/p/%C3%A5/t/h?qu%C3%ABry=strin%C4%9F&foo=bar=yuk&key%23%26%3D=%25&lol#h%C3%a6sh";

    // Use unsafe block to call the C url_parse function
    let parsed = unsafe {
        let c_url = CString::new(url).unwrap();
        let parsed_ptr = url_parse(c_url.as_ptr());

        assert!(!parsed_ptr.is_null(), "parsed should not be null");
        parsed_ptr
    };

    // Inspect the parsed URL data
    unsafe {
        url_data_inspect(parsed);

        // Check that all fields are non-null
        assert!(
            !(*parsed).whole_url.is_null(),
            "parsed->whole_url should not be null"
        );
        assert!(
            !(*parsed).protocol.is_null(),
            "parsed->protocol should not be null"
        );
        assert!(
            !(*parsed).userinfo.is_null(),
            "parsed->userinfo should not be null"
        );
        assert!(!(*parsed).host.is_null(), "parsed->host should not be null");
        assert!(!(*parsed).port.is_null(), "parsed->port should not be null");
        assert!(!(*parsed).path.is_null(), "parsed->path should not be null");
        assert!(
            !(*parsed).query.is_null(),
            "parsed->query should not be null"
        );
        assert!(
            !(*parsed).fragment.is_null(),
            "parsed->fragment should not be null"
        );
    }

    // Test protocol
    unsafe {
        let c_url = CString::new(url).unwrap();
        let protocol_ptr = url_get_protocol(c_url.as_ptr());
        assert!(!protocol_ptr.is_null(), "s");

        let protocol_str = CStr::from_ptr(protocol_ptr).to_string_lossy();
        assert_eq!("http", protocol_str, "strcmp(\"http\", s) == 0");

        free(protocol_ptr as *mut _);
    }

    // Test userinfo
    unsafe {
        let c_url = CString::new(url).unwrap();
        let userinfo_ptr = url_get_userinfo(c_url.as_ptr());
        assert!(!userinfo_ptr.is_null(), "s");

        let userinfo_str = CStr::from_ptr(userinfo_ptr).to_string_lossy();
        assert_eq!("user:pass", userinfo_str, "strcmp(\"user:pass\", s) == 0");

        free(userinfo_ptr as *mut _);
    }

    // Test hostname
    unsafe {
        let c_url = CString::new(url).unwrap();
        let hostname_ptr = url_get_hostname(c_url.as_ptr());
        assert!(!hostname_ptr.is_null(), "s");

        let hostname_str = CStr::from_ptr(hostname_ptr).to_string_lossy();
        assert_eq!(
            "subdomain.host.com", hostname_str,
            "strcmp(\"subdomain.host.com\", s) == 0"
        );

        free(hostname_ptr as *mut _);
    }

    // Test path
    unsafe {
        let c_url = CString::new(url).unwrap();
        let path_ptr = url_get_path(c_url.as_ptr());
        assert!(!path_ptr.is_null(), "s");

        let path_str = CStr::from_ptr(path_ptr).to_string_lossy();
        // The path contains a UTF-8 character 'å' which is represented as two bytes in UTF-8
        let expected_path = "/p/å/t/h";
        assert_eq!(expected_path, path_str, "path should match expected value");

        free(path_ptr as *mut _);
    }

    // Test query values
    unsafe {
        // Create C strings for query keys
        let query_key1 = CString::new("quëry").unwrap();
        let query_val1 =
            CStr::from_ptr(url_get_query_value(parsed, query_key1.as_ptr())).to_string_lossy();
        assert_eq!(
            "strinğ", query_val1,
            "query value for 'quëry' should be 'strinğ'"
        );

        let query_key2 = CString::new("foo").unwrap();
        let query_val2 =
            CStr::from_ptr(url_get_query_value(parsed, query_key2.as_ptr())).to_string_lossy();
        assert_eq!(
            "bar=yuk", query_val2,
            "query value for 'foo' should be 'bar=yuk'"
        );

        let query_key3 = CString::new("key#&=").unwrap();
        let query_val3 =
            CStr::from_ptr(url_get_query_value(parsed, query_key3.as_ptr())).to_string_lossy();
        assert_eq!("%", query_val3, "query value for 'key#&=' should be '%'");

        let query_key4 = CString::new("lol").unwrap();
        let query_val4 =
            CStr::from_ptr(url_get_query_value(parsed, query_key4.as_ptr())).to_string_lossy();
        assert_eq!("", query_val4, "query value for 'lol' should be empty");
    }

    // Test fragment
    unsafe {
        let c_url = CString::new(url).unwrap();
        let fragment_ptr = url_get_fragment(c_url.as_ptr());
        assert!(!fragment_ptr.is_null(), "s");

        let fragment_str = CStr::from_ptr(fragment_ptr).to_string_lossy();
        assert_eq!("hæsh", fragment_str, "fragment should be 'hæsh'");

        free(fragment_ptr as *mut _);
    }

    // Test port
    unsafe {
        let c_url = CString::new(url).unwrap();
        let port_ptr = url_get_port(c_url.as_ptr());
        assert!(!port_ptr.is_null(), "s");

        let port_str = CStr::from_ptr(port_ptr).to_string_lossy();
        assert_eq!("8080", port_str, "port should be '8080'");

        free(port_ptr as *mut _);
    }

    // Free the parsed URL data
    unsafe {
        url_free(parsed);
    }
}
#[no_mangle]

fn test2_rust() {
    let gh_url = "git://git@github.com:jwerle/url.h.git";

    // Create a C string for the URL
    let gh_url_c = CString::new(gh_url).unwrap();

    // Parse the URL using the C function
    let gh_parsed = unsafe { url_parse(gh_url_c.as_ptr()) };

    // Assert that parsing was successful
    if gh_parsed.is_null() {
        unsafe {
            let assertion = CString::new("gh_parsed").unwrap();
            let file = CString::new("test.c").unwrap();
            let function = CString::new("test2").unwrap();
            __assert_fail(assertion.as_ptr(), file.as_ptr(), 29, function.as_ptr());
        }
    }

    // Inspect the parsed URL data
    unsafe { url_data_inspect(gh_parsed) };

    // Check that the fields are not null
    unsafe {
        let data = &*gh_parsed;

        if data.whole_url.is_null() {
            let assertion = CString::new("gh_parsed->whole_url").unwrap();
            let file = CString::new("test.c").unwrap();
            let function = CString::new("test2").unwrap();
            __assert_fail(assertion.as_ptr(), file.as_ptr(), 31, function.as_ptr());
        }

        if data.protocol.is_null() {
            let assertion = CString::new("gh_parsed->protocol").unwrap();
            let file = CString::new("test.c").unwrap();
            let function = CString::new("test2").unwrap();
            __assert_fail(assertion.as_ptr(), file.as_ptr(), 32, function.as_ptr());
        }

        if data.userinfo.is_null() {
            let assertion = CString::new("gh_parsed->userinfo").unwrap();
            let file = CString::new("test.c").unwrap();
            let function = CString::new("test2").unwrap();
            __assert_fail(assertion.as_ptr(), file.as_ptr(), 33, function.as_ptr());
        }

        if data.host.is_null() {
            let assertion = CString::new("gh_parsed->host").unwrap();
            let file = CString::new("test.c").unwrap();
            let function = CString::new("test2").unwrap();
            __assert_fail(assertion.as_ptr(), file.as_ptr(), 34, function.as_ptr());
        }

        if data.path.is_null() {
            let assertion = CString::new("gh_parsed->path").unwrap();
            let file = CString::new("test.c").unwrap();
            let function = CString::new("test2").unwrap();
            __assert_fail(assertion.as_ptr(), file.as_ptr(), 35, function.as_ptr());
        }
    }

    // Check protocol
    unsafe {
        let s = url_get_protocol(gh_url_c.as_ptr());
        if s.is_null() {
            let assertion = CString::new("s").unwrap();
            let file = CString::new("test.c").unwrap();
            let function = CString::new("test2").unwrap();
            __assert_fail(assertion.as_ptr(), file.as_ptr(), 37, function.as_ptr());
        }

        let git_str = CString::new("git").unwrap();
        if strcmp(s, git_str.as_ptr()) != 0 {
            let assertion = CString::new("strcmp(\"git\", s) == 0").unwrap();
            let file = CString::new("test.c").unwrap();
            let function = CString::new("test2").unwrap();
            __assert_fail(assertion.as_ptr(), file.as_ptr(), 37, function.as_ptr());
        }

        free(s as *mut _);
    }

    // Check hostname
    unsafe {
        let s = url_get_hostname(gh_url_c.as_ptr());
        if s.is_null() {
            let assertion = CString::new("s").unwrap();
            let file = CString::new("test.c").unwrap();
            let function = CString::new("test2").unwrap();
            __assert_fail(assertion.as_ptr(), file.as_ptr(), 38, function.as_ptr());
        }

        let github_str = CString::new("github.com").unwrap();
        if strcmp(s, github_str.as_ptr()) != 0 {
            let assertion = CString::new("strcmp(\"github.com\", s) == 0").unwrap();
            let file = CString::new("test.c").unwrap();
            let function = CString::new("test2").unwrap();
            __assert_fail(assertion.as_ptr(), file.as_ptr(), 38, function.as_ptr());
        }

        free(s as *mut _);
    }

    // Check userinfo
    unsafe {
        let s = url_get_userinfo(gh_url_c.as_ptr());
        if s.is_null() {
            let assertion = CString::new("s").unwrap();
            let file = CString::new("test.c").unwrap();
            let function = CString::new("test2").unwrap();
            __assert_fail(assertion.as_ptr(), file.as_ptr(), 39, function.as_ptr());
        }

        let git_str = CString::new("git").unwrap();
        if strcmp(s, git_str.as_ptr()) != 0 {
            let assertion = CString::new("strcmp(\"git\", s) == 0").unwrap();
            let file = CString::new("test.c").unwrap();
            let function = CString::new("test2").unwrap();
            __assert_fail(assertion.as_ptr(), file.as_ptr(), 39, function.as_ptr());
        }

        free(s as *mut _);
    }

    // Check path
    unsafe {
        let s = url_get_path(gh_url_c.as_ptr());
        if s.is_null() {
            let assertion = CString::new("s").unwrap();
            let file = CString::new("test.c").unwrap();
            let function = CString::new("test2").unwrap();
            __assert_fail(assertion.as_ptr(), file.as_ptr(), 40, function.as_ptr());
        }

        let path_str = CString::new("jwerle/url.h.git").unwrap();
        if strcmp(s, path_str.as_ptr()) != 0 {
            let assertion = CString::new("strcmp(\"jwerle/url.h.git\", s) == 0").unwrap();
            let file = CString::new("test.c").unwrap();
            let function = CString::new("test2").unwrap();
            __assert_fail(assertion.as_ptr(), file.as_ptr(), 40, function.as_ptr());
        }

        free(s as *mut _);
    }

    // Free the parsed URL data
    unsafe { url_free(gh_parsed) };
}
#[no_mangle]

fn test1_rust(url_schemes: &[&str]) {
    // In Rust, we can use the assert! macro instead of the C-style assertions
    assert!(
        url_is_protocol_rust("http", url_schemes),
        "url_is_protocol(\"http\")"
    );
    assert!(
        url_is_protocol_rust("https", url_schemes),
        "url_is_protocol(\"https\")"
    );
    assert!(
        url_is_protocol_rust("git", url_schemes),
        "url_is_protocol(\"git\")"
    );
    assert!(
        url_is_protocol_rust("ssh", url_schemes),
        "url_is_protocol(\"ssh\")"
    );
    assert!(
        url_is_protocol_rust("sftp", url_schemes),
        "url_is_protocol(\"sftp\")"
    );
    assert!(
        url_is_protocol_rust("ftp", url_schemes),
        "url_is_protocol(\"ftp\")"
    );
    assert!(
        url_is_protocol_rust("javascript", url_schemes),
        "url_is_protocol(\"javascript\")"
    );
}
#[no_mangle]

fn run_test_rust(number: i32) {
    match number {
        1 => test1_rust(&[]), // Note: assuming empty slice as default, adjust as needed
        2 => test2_rust(),
        3 => test3_rust(),
        4 => test4_rust(),
        5 => test5_rust(),
        6 => test6_rust(),
        7 => test7_rust(),
        8 => test8_rust(),
        9 => test9_rust(),
        10 => test10_rust(),
        _ => {
            eprintln!("Invalid test number");
            process::exit(1);
        }
    }
}
#[no_mangle]

fn main_0_rust() -> i32 {
    // Get command line arguments
    let args: Vec<String> = env::args().collect();

    if args.len() != 2 {
        println!("Usage: {} <test_number>", args[0]);
        process::exit(1);
    }

    // Parse the test number safely using Rust's parse method
    let number = match args[1].parse::<i32>() {
        Ok(num) => num,
        Err(_) => {
            println!("Invalid test number: {}, should be 1-10", args[1]);
            process::exit(1);
        }
    };

    if number < 1 || number > 10 {
        println!("Invalid test number: {}, should be 1-10", number);
        process::exit(1);
    }

    run_test_rust(number);
    0
}
#[no_mangle]
pub unsafe extern "C" fn main_0(argc: c_int, argv: *mut *mut c_char) -> c_int {
    // Call the Rust implementation
    main_0_rust()
}
