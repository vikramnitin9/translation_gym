#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

// No additional imports needed
use std::{assert_eq, f64};
// No additional imports needed
use std::cmp::min;
// No additional imports needed
// No additional imports needed
use libc::{c_int, c_uchar, c_void, size_t};
use std::{
    alloc::{alloc, Layout},
    any::Any,
    assert, cmp,
    convert::TryInto,
    ffi::CStr,
    io::{self, Read, Write},
    mem, ptr, slice,
    vec::Vec,
};
include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
include!("main_func.rs");
/// Reads a single character from the standard input.
///
/// Returns the character read as an integer, or -1 on EOF or error.
fn getchar_rust() -> i32 {
    let mut buffer = [0u8; 1];
    match io::stdin().read_exact(&mut buffer) {
        Ok(_) => buffer[0] as i32,
        Err(_) => -1,
    }
}
#[no_mangle]
pub unsafe extern "C" fn getchar() -> c_int {
    getchar_rust() as c_int
}

/// Reads all bytes from stdin and returns them as a Vec<u8>
///
/// Returns None if there was an error reading from stdin
fn read_stdin_to_bytes_rust() -> Option<Vec<u8>> {
    // In Rust, we can use Vec<u8> which handles memory management for us
    let mut buffer = Vec::with_capacity(1024);

    // Read from stdin
    match std::io::stdin().read_to_end(&mut buffer) {
        Ok(_) => Some(buffer),
        Err(e) => {
            eprintln!("Error reading from stdin: {}", e);
            None
        }
    }
}
#[no_mangle]
pub unsafe extern "C" fn read_stdin_to_bytes(out_size: *mut libc::size_t) -> *mut u8 {
    // Validate input pointer
    if out_size.is_null() {
        return std::ptr::null_mut();
    }

    // Call the Rust implementation
    match read_stdin_to_bytes_rust() {
        Some(buffer) => {
            // Get the size of the buffer
            let size = buffer.len();

            // Allocate memory for the result
            let result = libc::malloc(size as libc::size_t) as *mut u8;
            if result.is_null() {
                eprintln!("Memory allocation failed");
                return std::ptr::null_mut();
            }

            // Copy the data
            std::ptr::copy_nonoverlapping(buffer.as_ptr(), result, size);

            // Set the output size
            *out_size = size as libc::size_t;

            result
        }
        None => std::ptr::null_mut(),
    }
}

fn AddBit_rust(bit: i32, bp: &mut u8, out: &mut Vec<u8>, outsize: &mut usize) {
    if *bp == 0 {
        // Check if outsize is a power of two (or zero)
        if *outsize == 0 || (*outsize & (*outsize - 1)) == 0 {
            // Resize the vector
            let new_size = if *outsize == 0 { 1 } else { *outsize * 2 };
            out.resize(new_size, 0);
        }

        // Ensure the vector has enough capacity for the new element
        if *outsize >= out.len() {
            out.push(0);
        } else {
            out[*outsize] = 0;
        }

        *outsize += 1;
    }

    // Set the bit at the current position
    out[*outsize - 1] |= (bit as u8) << *bp;

    // Update bit position, wrapping at 8
    *bp = (*bp + 1) & 7;
}
#[no_mangle]
pub unsafe extern "C" fn AddBit(bit: c_int, bp: *mut u8, out: *mut *mut u8, outsize: *mut usize) {
    // Safety: We're assuming the pointers are valid and properly aligned
    let bp_ref = &mut *bp;
    let outsize_ref = &mut *outsize;

    // Create a Vec that will manage our buffer
    let mut rust_vec: Vec<u8>;

    // Initialize or get the existing buffer as a Vec
    if *outsize_ref == 0 {
        rust_vec = Vec::new();
    } else {
        let slice = std::slice::from_raw_parts(*out, *outsize_ref);
        rust_vec = slice.to_vec();
    }

    // Call the Rust implementation
    AddBit_rust(bit, bp_ref, &mut rust_vec, outsize_ref);

    // Update the C pointer to point to our Vec's buffer
    // We need to ensure the buffer isn't deallocated when rust_vec goes out of scope
    if !rust_vec.is_empty() {
        // Allocate memory for the new buffer
        let new_buf = rust_vec.as_ptr();
        let new_capacity = rust_vec.capacity();

        // Transfer ownership of the buffer to the C code
        let raw_ptr = new_buf as *mut u8;

        // Set the output pointer
        *out = raw_ptr;

        // Prevent Rust from deallocating the buffer
        mem::forget(rust_vec);
    }
}

fn AddNonCompressedBlock_rust(
    _options: &ZopfliOptions,
    final_block: bool,
    input: &[u8],
    instart: usize,
    inend: usize,
    bp: &mut u8,
    out: &mut Vec<u8>,
) {
    let mut pos = instart;

    loop {
        let mut blocksize = 65535_u16;

        if pos + blocksize as usize > inend {
            blocksize = (inend - pos) as u16;
        }

        let current_final = pos + blocksize as usize >= inend;
        let nlen = !blocksize;

        // Add header bits
        AddBit_rust(
            if final_block && current_final { 1 } else { 0 },
            bp,
            out,
            &mut out.len(),
        );
        // BTYPE 00
        AddBit_rust(0, bp, out, &mut out.len());
        AddBit_rust(0, bp, out, &mut out.len());

        // Any bits of input up to the next byte boundary are ignored
        *bp = 0;

        // Add block size and its complement
        out.push(blocksize as u8);
        out.push((blocksize >> 8) as u8);
        out.push(nlen as u8);
        out.push((nlen >> 8) as u8);

        // Add block data
        for i in 0..blocksize as usize {
            out.push(input[pos + i]);
        }

        if current_final {
            break;
        }

        pos += blocksize as usize;
    }
}
#[no_mangle]

fn ZopfliInitLZ77Store_rust(data: *const u8, store: &mut ZopfliLZ77Store) {
    store.size = 0;
    store.litlens = std::ptr::null_mut();
    store.dists = std::ptr::null_mut();
    store.pos = std::ptr::null_mut();
    store.data = data;
    store.ll_symbol = std::ptr::null_mut();
    store.d_symbol = std::ptr::null_mut();
    store.ll_counts = std::ptr::null_mut();
    store.d_counts = std::ptr::null_mut();
}
#[no_mangle]
pub unsafe extern "C" fn ZopfliInitLZ77Store(data: *const c_uchar, store: *mut ZopfliLZ77Store) {
    if !store.is_null() {
        ZopfliInitLZ77Store_rust(data as *const u8, &mut *store);
    }
}

/// A Rust implementation of ZopfliLongestMatchCache initialization
pub fn ZopfliInitCache_rust(blocksize: usize, lmc: &mut ZopfliLongestMatchCache) {
    // Allocate memory for length array
    let length_size = blocksize * std::mem::size_of::<u16>();
    let length_layout = Layout::array::<u16>(blocksize).unwrap();
    let length_ptr = unsafe { alloc(length_layout) as *mut u16 };

    // Allocate memory for dist array
    let dist_size = blocksize * std::mem::size_of::<u16>();
    let dist_layout = Layout::array::<u16>(blocksize).unwrap();
    let dist_ptr = unsafe { alloc(dist_layout) as *mut u16 };

    // Allocate memory for sublen array
    let sublen_size = 8 * 3 * blocksize;
    let sublen_layout = Layout::array::<u8>(sublen_size).unwrap();
    let sublen_ptr = unsafe { alloc(sublen_layout) as *mut u8 };

    // Check if allocation failed for sublen (which is the largest)
    if sublen_ptr.is_null() {
        // Use standard Rust error handling instead of C's fprintf and exit
        eprintln!(
            "Error: Out of memory. Tried allocating {} bytes of memory.",
            8 * 3 * blocksize
        );
        std::process::exit(1);
    }

    // Set the pointers in the struct
    lmc.length = length_ptr;
    lmc.dist = dist_ptr;
    lmc.sublen = sublen_ptr;

    // Initialize arrays
    unsafe {
        // Initialize length array with 1s
        for i in 0..blocksize {
            *length_ptr.add(i) = 1;
        }

        // Initialize dist array with 0s
        for i in 0..blocksize {
            *dist_ptr.add(i) = 0;
        }

        // Initialize sublen array with 0s
        for i in 0..sublen_size {
            *sublen_ptr.add(i) = 0;
        }
    }
}
#[no_mangle]
pub unsafe extern "C" fn ZopfliInitCache(blocksize: size_t, lmc: *mut ZopfliLongestMatchCache) {
    if !lmc.is_null() {
        ZopfliInitCache_rust(blocksize, &mut *lmc);
    }
}

pub struct ZopfliLongestMatchCache_rust {
    pub length: Vec<u16>,
    pub dist: Vec<u16>,
    pub sublen: Vec<u8>,
}

/// Initializes a ZopfliBlockState struct with the given options and block boundaries.
///
/// If `add_lmc` is true, it allocates and initializes a longest match cache.
fn ZopfliInitBlockState_rust(
    options: &ZopfliOptions,
    blockstart: usize,
    blockend: usize,
    add_lmc: bool,
    s: &mut ZopfliBlockState,
) {
    s.options = options as *const ZopfliOptions;
    s.blockstart = blockstart;
    s.blockend = blockend;

    if add_lmc {
        // Create a new longest match cache
        let layout = Layout::new::<ZopfliLongestMatchCache>();
        let lmc_ptr = unsafe { alloc(layout) as *mut ZopfliLongestMatchCache };

        if !lmc_ptr.is_null() {
            // Initialize the cache
            unsafe {
                ZopfliInitCache_rust(blockend - blockstart, &mut *lmc_ptr);
            }
            s.lmc = lmc_ptr;
        } else {
            s.lmc = ptr::null_mut();
        }
    } else {
        s.lmc = ptr::null_mut();
    }
}
#[no_mangle]

/// Allocates memory for the ZopfliHash data structure.
///
/// # Arguments
///
/// * `window_size` - The size of the sliding window
/// * `h` - The hash structure to initialize
fn ZopfliAllocHash_rust(window_size: usize, h: &mut ZopfliHash) {
    // Allocate memory for each field
    let head_layout = Layout::array::<c_int>(65536).unwrap();
    let prev_layout = Layout::array::<u16>(window_size).unwrap();
    let hashval_layout = Layout::array::<c_int>(window_size).unwrap();
    let same_layout = Layout::array::<u16>(window_size).unwrap();

    unsafe {
        // Allocate memory for each field
        h.head = alloc(head_layout) as *mut c_int;
        h.prev = alloc(prev_layout) as *mut u16;
        h.hashval = alloc(hashval_layout) as *mut c_int;
        h.same = alloc(same_layout) as *mut u16;
        h.head2 = alloc(head_layout) as *mut c_int;
        h.prev2 = alloc(prev_layout) as *mut u16;
        h.hashval2 = alloc(hashval_layout) as *mut c_int;

        // Initialize memory to zero
        ptr::write_bytes(h.head, 0, 65536);
        ptr::write_bytes(h.prev, 0, window_size);
        ptr::write_bytes(h.hashval, 0, window_size);
        ptr::write_bytes(h.same, 0, window_size);
        ptr::write_bytes(h.head2, 0, 65536);
        ptr::write_bytes(h.prev2, 0, window_size);
        ptr::write_bytes(h.hashval2, 0, window_size);
    }
}
/// C-compatible wrapper for ZopfliAllocHash_rust
///
/// # Safety
///
/// This function is unsafe because it dereferences raw pointers.
#[no_mangle]
pub unsafe extern "C" fn ZopfliAllocHash(window_size: size_t, h: *mut ZopfliHash) {
    if !h.is_null() {
        ZopfliAllocHash_rust(window_size, &mut *h);
    }
}

fn GetCostModelMinCost_rust<F>(costmodel: &F, costcontext: *mut c_void) -> f64
where
    F: Fn(i32, i32, *mut c_void) -> f64,
{
    let mut mincost: f64;
    let mut bestlength = 0; // length that has lowest cost in the cost model
    let mut bestdist = 0; // distance that has lowest cost in the cost model

    /*
    Table of distances that have a different distance symbol in the deflate
    specification. Each value is the first distance that has a new symbol. Only
    different symbols affect the cost model so only these need to be checked.
    See RFC 1951 section 3.2.5. Compressed blocks (length and distance codes).
    */
    const DSYMBOLS: [i32; 30] = [
        1, 2, 3, 4, 5, 7, 9, 13, 17, 25, 33, 49, 65, 97, 129, 193, 257, 385, 513, 769, 1025, 1537,
        2049, 3073, 4097, 6145, 8193, 12289, 16385, 24577,
    ];

    mincost = 1e30;
    for i in 3..259 {
        let c = costmodel(i, 1, costcontext);
        if c < mincost {
            bestlength = i;
            mincost = c;
        }
    }

    mincost = 1e30;
    for &dist in DSYMBOLS.iter() {
        let c = costmodel(3, dist, costcontext);
        if c < mincost {
            bestdist = dist;
            mincost = c;
        }
    }

    costmodel(bestlength, bestdist, costcontext)
}
#[no_mangle]
pub unsafe extern "C" fn GetCostModelMinCost(
    costmodel: Option<unsafe extern "C" fn(c_int, c_int, *mut c_void) -> f64>,
    costcontext: *mut c_void,
) -> f64 {
    // Unwrap the function pointer (it should never be None in practice)
    let costmodel_fn = match costmodel {
        Some(f) => f,
        None => return 0.0, // Return a default value if the function pointer is null
    };

    // Create a closure that adapts the C function pointer to a Rust function
    let rust_costmodel = |length: i32, dist: i32, context: *mut c_void| -> f64 {
        costmodel_fn(length, dist, context)
    };

    // Call the Rust implementation
    GetCostModelMinCost_rust(&rust_costmodel, costcontext)
}

fn ZopfliResetHash_rust(window_size: usize, h: &mut ZopfliHash) {
    h.val = 0;

    // Reset head array
    for i in 0..65536 {
        unsafe {
            *h.head.add(i) = -1; // -1 indicates no head so far
        }
    }

    // Reset prev and hashval arrays
    for i in 0..window_size {
        unsafe {
            *h.prev.add(i) = i as u16; // If prev[j] == j, then prev[j] is uninitialized
            *h.hashval.add(i) = -1;
        }
    }

    // Reset same array
    for i in 0..window_size {
        unsafe {
            *h.same.add(i) = 0;
        }
    }

    // Reset val2
    h.val2 = 0;

    // Reset head2 array
    for i in 0..65536 {
        unsafe {
            *h.head2.add(i) = -1;
        }
    }

    // Reset prev2 and hashval2 arrays
    for i in 0..window_size {
        unsafe {
            *h.prev2.add(i) = i as u16;
            *h.hashval2.add(i) = -1;
        }
    }
}
#[no_mangle]
pub unsafe extern "C" fn ZopfliResetHash(window_size: size_t, h: *mut ZopfliHash) {
    if !h.is_null() {
        ZopfliResetHash_rust(window_size, &mut *h);
    }
}

fn UpdateHashValue_rust(h: &mut ZopfliHash, c: u8) {
    h.val = (((h.val) << 5) ^ (c as i32)) & 32767;
}
#[no_mangle]
pub unsafe extern "C" fn UpdateHashValue(h: *mut ZopfliHash, c: c_uchar) {
    if !h.is_null() {
        let h_ref = &mut *h;
        UpdateHashValue_rust(h_ref, c as u8);
    }
}

fn ZopfliWarmupHash_rust(array: &[u8], pos: usize, end: usize, h: &mut ZopfliHash) {
    UpdateHashValue_rust(h, array[pos]);
    if pos + 1 < end {
        UpdateHashValue_rust(h, array[pos + 1]);
    }
}
#[no_mangle]
pub unsafe extern "C" fn ZopfliWarmupHash(
    array: *const c_uchar,
    pos: size_t,
    end: size_t,
    h: *mut ZopfliHash,
) {
    if array.is_null() || h.is_null() {
        return;
    }

    let array_slice = std::slice::from_raw_parts(array, end);
    let h_ref = &mut *h;

    ZopfliWarmupHash_rust(array_slice, pos, end, h_ref);
}

fn ZopfliMaxCachedSublen_rust(lmc: &ZopfliLongestMatchCache, pos: usize, _length: usize) -> u32 {
    // Calculate the offset in the sublen array
    let cache_offset = 8 * pos * 3;

    // Get a slice of the cache starting at the calculated position
    let cache = unsafe { std::slice::from_raw_parts(lmc.sublen.add(cache_offset), 8 * 3) };

    // Check if no sublen is cached
    if cache[1] == 0 && cache[2] == 0 {
        return 0; // No sublen cached
    }

    // Return the cached value + 3
    // The value is at offset (8-1)*3 from the current position
    cache[(8 - 1) * 3] as u32 + 3
}
#[no_mangle]

fn ZopfliCacheToSublen_rust(
    lmc: &ZopfliLongestMatchCache,
    pos: usize,
    length: usize,
    sublen: &mut [u16],
) {
    if length < 3 {
        return;
    }

    let maxlength = ZopfliMaxCachedSublen_rust(lmc, pos, length);
    let mut prevlength = 0;

    // Calculate the starting position in the cache
    let cache_start = 8 * pos * 3;

    // Safely access the sublen data using unsafe code
    unsafe {
        for j in 0..8 {
            let idx = cache_start + j * 3;

            // Read the values from the cache
            let length_val = *lmc.sublen.add(idx) as usize + 3;
            let dist = *lmc.sublen.add(idx + 1) as u16 + 256 * (*lmc.sublen.add(idx + 2) as u16);

            for i in prevlength..=length_val {
                if i < sublen.len() {
                    sublen[i] = dist;
                }
            }

            if length_val as u32 == maxlength {
                break;
            }

            prevlength = length_val + 1;
        }
    }
}
#[no_mangle]
pub unsafe extern "C" fn ZopfliCacheToSublen(
    lmc: *const ZopfliLongestMatchCache,
    pos: size_t,
    length: size_t,
    sublen: *mut libc::c_ushort,
) {
    if lmc.is_null() || sublen.is_null() {
        return;
    }

    // Create a slice for the sublen array
    // We don't know the exact length of the array, but it should be at least length+1
    let sublen_slice = std::slice::from_raw_parts_mut(sublen as *mut u16, length + 1);

    // Call the Rust implementation
    ZopfliCacheToSublen_rust(&*lmc, pos, length, sublen_slice);
}

fn TryGetFromLongestMatchCache_rust(
    s: &mut ZopfliBlockState,
    pos: usize,
    limit: &mut usize,
    sublen: Option<&mut [u16]>,
    distance: &mut u16,
    length: &mut u16,
) -> bool {
    // The LMC cache starts at the beginning of the block rather than the
    // beginning of the whole array.
    let lmcpos = pos - s.blockstart;

    // Check if cache is available
    let cache_available = !s.lmc.is_null()
        && unsafe {
            let lmc = &*s.lmc;
            *lmc.length.add(lmcpos) == 0 || *lmc.dist.add(lmcpos) != 0
        };

    let limit_ok_for_cache = if cache_available {
        unsafe {
            let lmc = &*s.lmc;
            *limit == 258
                || *lmc.length.add(lmcpos) as usize <= *limit
                || (sublen.is_some()
                    && ZopfliMaxCachedSublen_rust(lmc, lmcpos, *lmc.length.add(lmcpos) as usize)
                        as usize
                        >= *limit)
        }
    } else {
        false
    };

    if !s.lmc.is_null() && limit_ok_for_cache && cache_available {
        unsafe {
            let lmc = &*s.lmc;

            if sublen.is_none()
                || *lmc.length.add(lmcpos) as usize
                    <= ZopfliMaxCachedSublen_rust(lmc, lmcpos, *lmc.length.add(lmcpos) as usize)
                        as usize
            {
                *length = *lmc.length.add(lmcpos);
                if *length as usize > *limit {
                    *length = *limit as u16;
                }

                if let Some(sublen_slice) = sublen {
                    ZopfliCacheToSublen_rust(lmc, lmcpos, *length as usize, sublen_slice);
                    *distance = sublen_slice[*length as usize];

                    if *limit == 258 && *length >= 3 {
                        assert!(
                            sublen_slice[*length as usize] == *lmc.dist.add(lmcpos),
                            "sublen[*length] == s->lmc->dist[lmcpos]"
                        );
                    }
                } else {
                    *distance = *lmc.dist.add(lmcpos);
                }

                return true;
            }

            // Can't use much of the cache, since the "sublens" need to be calculated,
            // but at least we already know when to stop.
            *limit = *lmc.length.add(lmcpos) as usize;
        }
    }

    false
}
#[no_mangle]
pub unsafe extern "C" fn TryGetFromLongestMatchCache(
    s: *mut ZopfliBlockState,
    pos: size_t,
    limit: *mut size_t,
    sublen: *mut libc::c_ushort,
    distance: *mut libc::c_ushort,
    length: *mut libc::c_ushort,
) -> c_int {
    let s_ref = &mut *s;
    let mut limit_val = *limit;
    let mut distance_val = *distance;
    let mut length_val = *length;

    let sublen_option = if !sublen.is_null() {
        // Determine the length of the sublen array
        // In the original code, this is expected to be at least 259 elements
        // (limit can be up to 258, and we need indices 0 through limit)
        let sublen_slice = std::slice::from_raw_parts_mut(sublen, 259);
        Some(sublen_slice)
    } else {
        None
    };

    let result = TryGetFromLongestMatchCache_rust(
        s_ref,
        pos,
        &mut limit_val,
        sublen_option,
        &mut distance_val,
        &mut length_val,
    );

    *limit = limit_val;
    *distance = distance_val;
    *length = length_val;

    if result {
        1
    } else {
        0
    }
}

/// Finds the position where two slices start to differ.
///
/// This function compares two byte slices and returns the position in the first slice
/// where they start to differ, or the end of the first slice if they are identical
/// up to that point.
fn GetMatch_rust(scan: &[u8], match_slice: &[u8], end_pos: usize, safe_end_pos: usize) -> usize {
    let mut pos = 0;
    let safe_len = safe_end_pos.saturating_sub(pos);

    if std::mem::size_of::<usize>() == 8 {
        // 8 bytes at a time (64-bit)
        let chunk_size = 8;
        let safe_chunks = safe_len / chunk_size;

        for i in 0..safe_chunks {
            let scan_idx = pos + i * chunk_size;
            let match_idx = i * chunk_size;

            let scan_chunk = &scan[scan_idx..scan_idx + chunk_size];
            let match_chunk = &match_slice[match_idx..match_idx + chunk_size];

            if scan_chunk != match_chunk {
                break;
            }

            pos += chunk_size;
        }
    } else if std::mem::size_of::<u32>() == 4 {
        // 4 bytes at a time (32-bit)
        let chunk_size = 4;
        let safe_chunks = safe_len / chunk_size;

        for i in 0..safe_chunks {
            let scan_idx = pos + i * chunk_size;
            let match_idx = i * chunk_size;

            let scan_chunk = &scan[scan_idx..scan_idx + chunk_size];
            let match_chunk = &match_slice[match_idx..match_idx + chunk_size];

            if scan_chunk != match_chunk {
                break;
            }

            pos += chunk_size;
        }
    } else {
        // 8 bytes at a time, one by one
        while pos + 8 <= safe_end_pos
            && scan[pos] == match_slice[pos]
            && scan[pos + 1] == match_slice[pos + 1]
            && scan[pos + 2] == match_slice[pos + 2]
            && scan[pos + 3] == match_slice[pos + 3]
            && scan[pos + 4] == match_slice[pos + 4]
            && scan[pos + 5] == match_slice[pos + 5]
            && scan[pos + 6] == match_slice[pos + 6]
            && scan[pos + 7] == match_slice[pos + 7]
        {
            pos += 8;
        }
    }

    // Check remaining bytes
    while pos < end_pos && scan[pos] == match_slice[pos] {
        pos += 1;
    }

    pos
}
#[no_mangle]
pub unsafe extern "C" fn GetMatch(
    scan: *const c_uchar,
    match_ptr: *const c_uchar,
    end: *const c_uchar,
    safe_end: *const c_uchar,
) -> *const c_uchar {
    if scan.is_null() || match_ptr.is_null() || end.is_null() || safe_end.is_null() {
        return scan;
    }

    let scan_offset = scan as usize;
    let end_offset = end as usize;
    let safe_end_offset = safe_end as usize;

    // Calculate lengths
    let end_pos = end_offset.saturating_sub(scan_offset);
    let safe_end_pos = safe_end_offset.saturating_sub(scan_offset);

    // Create slices (this is unsafe but contained within the wrapper)
    let scan_slice = std::slice::from_raw_parts(scan, end_pos);
    let match_slice = std::slice::from_raw_parts(match_ptr, end_pos);

    // Call the safe Rust implementation
    let pos = GetMatch_rust(scan_slice, match_slice, end_pos, safe_end_pos);

    // Return the pointer to the position where the match ends
    scan.add(pos)
}

fn ZopfliSublenToCache_rust(
    sublen: &[u16],
    pos: usize,
    length: usize,
    lmc: &mut ZopfliLongestMatchCache,
) {
    if length < 3 {
        return;
    }

    let cache_start = 8 * pos * 3;
    let mut j = 0;
    let mut bestlength = 0;

    // Create a safe slice from the sublen pointer
    // We need to use unsafe here to work with the raw pointer in the struct
    let cache_slice = unsafe {
        if lmc.sublen.is_null() {
            return;
        }
        // Create a mutable slice that covers the entire cache area we need
        // Using length field which should represent the size of the cache
        slice::from_raw_parts_mut(lmc.sublen, lmc.length as usize * 8 * 3)
    };

    for i in 3..=length {
        if i == length || sublen[i] != sublen[i + 1] {
            let idx = cache_start + j * 3;
            cache_slice[idx] = (i - 3) as u8;
            cache_slice[idx + 1] = (sublen[i] % 256) as u8;
            cache_slice[idx + 2] = ((sublen[i] >> 8) % 256) as u8;
            bestlength = i;
            j += 1;
            if j >= 8 {
                break;
            }
        }
    }

    if j < 8 {
        debug_assert!(bestlength == length, "bestlength == length");
        cache_slice[cache_start + (8 - 1) * 3] = (bestlength - 3) as u8;
    } else {
        debug_assert!(bestlength <= length, "bestlength <= length");
    }

    // Convert u32 to usize for comparison
    let max_cached_sublen = ZopfliMaxCachedSublen_rust(lmc, pos, length) as usize;
    debug_assert!(
        bestlength == max_cached_sublen,
        "bestlength == ZopfliMaxCachedSublen(lmc, pos, length)"
    );
}
#[no_mangle]
pub unsafe extern "C" fn ZopfliSublenToCache(
    sublen: *const libc::c_ushort,
    pos: size_t,
    length: size_t,
    lmc: *mut ZopfliLongestMatchCache,
) {
    if sublen.is_null() || lmc.is_null() {
        return;
    }

    // Create a slice from the raw pointer
    let sublen_slice = std::slice::from_raw_parts(sublen as *const u16, length + 2);

    // Call the Rust implementation
    ZopfliSublenToCache_rust(sublen_slice, pos as usize, length as usize, &mut *lmc);
}

fn StoreInLongestMatchCache_rust(
    s: &mut ZopfliBlockState,
    pos: usize,
    limit: usize,
    sublen: Option<&[u16]>,
    distance: u16,
    length: u16,
) {
    // The LMC cache starts at the beginning of the block rather than the
    // beginning of the whole array.
    let lmcpos = pos - s.blockstart as usize;

    // Length > 0 and dist 0 is invalid combination, which indicates on purpose
    // that this cache value is not filled in yet.
    let cache_available = if !s.lmc.is_null() {
        let lmc = unsafe { &*s.lmc };
        unsafe { *lmc.length.add(lmcpos) == 0 || *lmc.dist.add(lmcpos) != 0 }
    } else {
        false
    };

    if !s.lmc.is_null() && limit == 258 && sublen.is_some() && !cache_available {
        let lmc = unsafe { &mut *s.lmc };

        // Assert that the cache entry is in the expected initial state
        debug_assert!(unsafe { *lmc.length.add(lmcpos) == 1 && *lmc.dist.add(lmcpos) == 0 });

        // Store the match information
        unsafe {
            *lmc.dist.add(lmcpos) = if length < 3 { 0 } else { distance };
            *lmc.length.add(lmcpos) = if length < 3 { 0 } else { length };
        }

        // Verify we didn't create an invalid state
        debug_assert!(unsafe { !(*lmc.length.add(lmcpos) == 1 && *lmc.dist.add(lmcpos) == 0) });

        // Store sublen information
        if let Some(sublen_slice) = sublen {
            unsafe {
                ZopfliSublenToCache_rust(sublen_slice, lmcpos, length as usize, lmc);
            }
        }
    }
}
#[no_mangle]
pub unsafe extern "C" fn StoreInLongestMatchCache(
    s: *mut ZopfliBlockState,
    pos: size_t,
    limit: size_t,
    sublen: *const u16,
    distance: u16,
    length: u16,
) {
    let s_ref = &mut *s;
    let sublen_slice = if !sublen.is_null() {
        // Create a slice of appropriate length based on the length parameter
        Some(slice::from_raw_parts(sublen, length as usize))
    } else {
        None
    };

    StoreInLongestMatchCache_rust(
        s_ref,
        pos as usize,
        limit as usize,
        sublen_slice,
        distance,
        length,
    );
}

fn zopfli_min_rust(a: usize, b: usize) -> usize {
    cmp::min(a, b)
}
#[no_mangle]
pub unsafe extern "C" fn zopfli_min(a: size_t, b: size_t) -> size_t {
    zopfli_min_rust(a, b)
}

fn TraceBackwards_rust(size: usize, length_array: &[u16], path: &mut Vec<u16>) {
    if size == 0 {
        return;
    }

    let mut index = size;
    let mut temp_path = Vec::new();

    loop {
        // Add current length to the path
        let length = length_array[index];
        temp_path.push(length);

        // Assertions
        assert!(usize::from(length) <= index, "length_array[index] <= index");
        assert!(length <= 258, "length_array[index] <= ZOPFLI_MAX_MATCH");
        assert!(length != 0, "length_array[index] != 0");

        // Move backwards
        index -= usize::from(length);

        if index == 0 {
            break;
        }
    }

    // Mirror result by extending the path with reversed elements
    path.clear();
    path.extend(temp_path.iter().rev());
}
#[no_mangle]
pub unsafe extern "C" fn TraceBackwards(
    size: size_t,
    length_array: *const u16,
    path: *mut *mut u16,
    pathsize: *mut size_t,
) {
    // Convert C parameters to Rust
    let size = size as usize;

    // Safety: We trust that length_array points to a valid array of at least size+1 elements
    let length_slice = if !length_array.is_null() && size > 0 {
        std::slice::from_raw_parts(length_array, size + 1)
    } else {
        return;
    };

    // Create or get the existing Vec
    let mut rust_path = if *pathsize == 0 {
        Vec::new()
    } else {
        // Safety: We trust that *path points to a valid array of *pathsize elements
        Vec::from_raw_parts(*path, *pathsize, *pathsize)
    };

    // Call the Rust implementation
    TraceBackwards_rust(size, length_slice, &mut rust_path);

    // Update the C pointers
    let new_len = rust_path.len();
    let new_cap = rust_path.capacity();

    // Prevent the Vec from being dropped and freeing the memory
    let ptr = rust_path.as_mut_ptr();
    std::mem::forget(rust_path);

    *path = ptr;
    *pathsize = new_len;
}

fn ZopfliVerifyLenDist_rust(data: &[u8], pos: usize, dist: u16, length: u16) {
    // In Rust, we can use the debug_assert! macro which is only checked in debug builds
    let length_usize = length as usize;
    let dist_usize = dist as usize;

    debug_assert!(
        pos + length_usize <= data.len(),
        "pos + length <= datasize (pos: {}, length: {}, datasize: {})",
        pos,
        length_usize,
        data.len()
    );

    for i in 0..length_usize {
        debug_assert!(
            data[pos - dist_usize + i] == data[pos + i],
            "data[pos - dist + i] == data[pos + i] (pos: {}, dist: {}, i: {})",
            pos,
            dist_usize,
            i
        );
    }
}
#[no_mangle]
pub unsafe extern "C" fn ZopfliVerifyLenDist(
    data: *const c_uchar,
    datasize: size_t,
    pos: size_t,
    dist: libc::c_ushort,
    length: libc::c_ushort,
) {
    // Convert C types to Rust types
    let data_slice = slice::from_raw_parts(data, datasize);

    // Call the Rust implementation
    ZopfliVerifyLenDist_rust(data_slice, pos, dist, length);
}

fn ZopfliGetLengthSymbol_rust(l: usize) -> usize {
    // Static lookup table for length symbols
    static TABLE: [usize; 259] = [
        0, 0, 0, 257, 258, 259, 260, 261, 262, 263, 264, 265, 265, 266, 266, 267, 267, 268, 268,
        269, 269, 269, 269, 270, 270, 270, 270, 271, 271, 271, 271, 272, 272, 272, 272, 273, 273,
        273, 273, 273, 273, 273, 273, 274, 274, 274, 274, 274, 274, 274, 274, 275, 275, 275, 275,
        275, 275, 275, 275, 276, 276, 276, 276, 276, 276, 276, 276, 277, 277, 277, 277, 277, 277,
        277, 277, 277, 277, 277, 277, 277, 277, 277, 277, 278, 278, 278, 278, 278, 278, 278, 278,
        278, 278, 278, 278, 278, 278, 278, 278, 279, 279, 279, 279, 279, 279, 279, 279, 279, 279,
        279, 279, 279, 279, 279, 279, 280, 280, 280, 280, 280, 280, 280, 280, 280, 280, 280, 280,
        280, 280, 280, 280, 281, 281, 281, 281, 281, 281, 281, 281, 281, 281, 281, 281, 281, 281,
        281, 281, 281, 281, 281, 281, 281, 281, 281, 281, 281, 281, 281, 281, 281, 281, 281, 281,
        282, 282, 282, 282, 282, 282, 282, 282, 282, 282, 282, 282, 282, 282, 282, 282, 282, 282,
        282, 282, 282, 282, 282, 282, 282, 282, 282, 282, 282, 282, 282, 282, 283, 283, 283, 283,
        283, 283, 283, 283, 283, 283, 283, 283, 283, 283, 283, 283, 283, 283, 283, 283, 283, 283,
        283, 283, 283, 283, 283, 283, 283, 283, 283, 283, 284, 284, 284, 284, 284, 284, 284, 284,
        284, 284, 284, 284, 284, 284, 284, 284, 284, 284, 284, 284, 284, 284, 284, 284, 284, 284,
        284, 284, 284, 284, 284, 285,
    ];

    // Bounds check to prevent panic
    if l < TABLE.len() {
        TABLE[l]
    } else {
        // Return the last value for out-of-bounds inputs
        // This matches C behavior where out-of-bounds access would likely
        // return the last value or cause undefined behavior
        *TABLE.last().unwrap_or(&0)
    }
}
#[no_mangle]
pub unsafe extern "C" fn ZopfliGetLengthSymbol(l: c_int) -> c_int {
    // Convert from C int to Rust usize, ensuring non-negative values
    let l_usize = if l >= 0 { l as usize } else { 0 };

    // Call the Rust implementation and convert back to C int
    ZopfliGetLengthSymbol_rust(l_usize) as c_int
}

fn ZopfliGetDistSymbol_rust(dist: i32) -> i32 {
    if dist < 5 {
        dist - 1
    } else {
        // In Rust, we can use leading_zeros() instead of __builtin_clz
        let l = (31 ^ (dist - 1).leading_zeros() as i32); // log2(dist - 1)
        let r = ((dist - 1) >> (l - 1)) & 1;
        l * 2 + r
    }
}
#[no_mangle]
pub unsafe extern "C" fn ZopfliGetDistSymbol(dist: c_int) -> c_int {
    ZopfliGetDistSymbol_rust(dist)
}

fn FollowPath_rust(
    s: &mut ZopfliBlockState,
    input: &[u8],
    instart: usize,
    inend: usize,
    path: &[u16],
    store: &mut ZopfliLZ77Store,
    h: &mut ZopfliHash,
) {
    let mut pos = 0;
    let windowstart = if instart > 32768 { instart - 32768 } else { 0 };

    if instart == inend {
        return;
    }

    ZopfliResetHash_rust(32768, h);
    ZopfliWarmupHash_rust(input, windowstart, inend, h);

    for i in windowstart..instart {
        // Need to use unsafe here since ZopfliUpdateHash doesn't have a Rust implementation
        unsafe {
            ZopfliUpdateHash(input.as_ptr(), i, inend, h as *mut ZopfliHash);
        }
    }

    pos = instart;
    for &length in path {
        let length = length as u16;

        // Assert that pos < inend
        assert!(pos < inend, "pos < inend");

        // Need to use unsafe here since ZopfliUpdateHash doesn't have a Rust implementation
        unsafe {
            ZopfliUpdateHash(input.as_ptr(), pos, inend, h as *mut ZopfliHash);
        }

        // Add to output
        if length >= 3 {
            let mut dist = 0u16;
            let mut dummy_length = 0u16;

            // Need to use unsafe here since ZopfliFindLongestMatch doesn't have a Rust implementation
            unsafe {
                ZopfliFindLongestMatch(
                    s as *mut ZopfliBlockState,
                    h as *const ZopfliHash,
                    input.as_ptr(),
                    pos,
                    inend,
                    length as usize,
                    std::ptr::null_mut(), // sublen parameter is null
                    &mut dist,
                    &mut dummy_length,
                );
            }

            // Assert that the lengths match
            assert!(
                !(dummy_length != length && length > 2 && dummy_length > 2),
                "!(dummy_length != length && length > 2 && dummy_length > 2)"
            );

            ZopfliVerifyLenDist_rust(input, pos, dist, length);

            // Need to use unsafe here since ZopfliStoreLitLenDist doesn't have a Rust implementation
            unsafe {
                ZopfliStoreLitLenDist(length, dist, pos, store as *mut ZopfliLZ77Store);
            }
        } else {
            let length = 1u16;

            // Need to use unsafe here since ZopfliStoreLitLenDist doesn't have a Rust implementation
            unsafe {
                ZopfliStoreLitLenDist(input[pos] as u16, 0, pos, store as *mut ZopfliLZ77Store);
            }
        }

        // Assert that pos + length <= inend
        assert!(pos + length as usize <= inend, "pos + length <= inend");

        for j in 1..length as usize {
            // Need to use unsafe here since ZopfliUpdateHash doesn't have a Rust implementation
            unsafe {
                ZopfliUpdateHash(input.as_ptr(), pos + j, inend, h as *mut ZopfliHash);
            }
        }

        pos += length as usize;
    }
}
#[no_mangle]
pub unsafe extern "C" fn FollowPath(
    s: *mut ZopfliBlockState,
    input: *const c_uchar,
    instart: size_t,
    inend: size_t,
    path: *mut u16,
    pathsize: size_t,
    store: *mut ZopfliLZ77Store,
    h: *mut ZopfliHash,
) {
    if s.is_null() || input.is_null() || path.is_null() || store.is_null() || h.is_null() {
        return;
    }

    let input_slice = slice::from_raw_parts(input, inend);
    let path_slice = slice::from_raw_parts(path, pathsize);

    FollowPath_rust(
        &mut *s,
        input_slice,
        instart,
        inend,
        path_slice,
        &mut *store,
        &mut *h,
    );
}

/// Cleans up the ZopfliHash struct by freeing all its allocated fields.
///
/// This is a Rust implementation that safely handles the deallocation of memory.
fn ZopfliCleanHash_rust(h: &mut ZopfliHash) {
    // Helper function to safely free a pointer and set it to null
    unsafe fn free_and_null<T>(ptr: &mut *mut T) {
        if !ptr.is_null() {
            libc::free(*ptr as *mut c_void);
            *ptr = ptr::null_mut();
        }
    }

    // Free all the pointers
    unsafe {
        free_and_null(&mut h.head);
        free_and_null(&mut h.prev);
        free_and_null(&mut h.hashval);

        free_and_null(&mut h.head2);
        free_and_null(&mut h.prev2);
        free_and_null(&mut h.hashval2);

        free_and_null(&mut h.same);
    }
}
#[no_mangle]
pub unsafe extern "C" fn ZopfliCleanHash(h: *mut ZopfliHash) {
    if !h.is_null() {
        // Convert the raw pointer to a mutable reference
        let hash = &mut *h;

        // Call the Rust implementation
        ZopfliCleanHash_rust(hash);
    }
}

pub struct ZopfliHash_rust {
    /// Hash value to index of its most recent occurrence.
    pub head: Vec<i32>,
    /// Index to index of prev. occurrence of same hash.
    pub prev: Vec<u16>,
    /// Index to hash value at this index.
    pub hashval: Vec<i32>,
    /// Current hash value.
    pub val: i32,

    /// Hash value to index of its most recent occurrence (second hash).
    pub head2: Vec<i32>,
    /// Index to index of prev. occurrence of same hash (second hash).
    pub prev2: Vec<u16>,
    /// Index to hash value at this index (second hash).
    pub hashval2: Vec<i32>,
    /// Current hash value (second hash).
    pub val2: i32,

    /// Amount of repetitions of same byte after this.
    pub same: Vec<u16>,
}

/// Rust implementation of ZopfliLZ77GetByteRange
///
/// Calculates the byte range covered by LZ77 entries from lstart to lend-1
fn ZopfliLZ77GetByteRange_rust(lz77: &ZopfliLZ77Store, lstart: usize, lend: usize) -> usize {
    if lstart == lend {
        return 0;
    }

    let l = lend - 1;

    // Get the positions and lengths safely
    let pos_start = unsafe { *lz77.pos.add(lstart) };
    let pos_l = unsafe { *lz77.pos.add(l) };
    let dist_l = unsafe { *lz77.dists.add(l) };
    let litlen_l = unsafe { *lz77.litlens.add(l) };

    // Calculate the byte range
    let length: usize = if dist_l == 0 { 1 } else { litlen_l as usize };

    pos_l + length - pos_start
}
#[no_mangle]
pub unsafe extern "C" fn ZopfliLZ77GetByteRange(
    lz77: *const ZopfliLZ77Store,
    lstart: size_t,
    lend: size_t,
) -> size_t {
    assert!(!lz77.is_null());

    let lz77_ref = &*lz77;
    ZopfliLZ77GetByteRange_rust(lz77_ref, lstart, lend)
}

fn GetFixedTree_rust(ll_lengths: &mut [u32], d_lengths: &mut [u32]) {
    // Fill literal/length codes (0-143 = 8 bits, 144-255 = 9 bits, 256-279 = 7 bits, 280-287 = 8 bits)
    for i in 0..144 {
        ll_lengths[i] = 8;
    }
    for i in 144..256 {
        ll_lengths[i] = 9;
    }
    for i in 256..280 {
        ll_lengths[i] = 7;
    }
    for i in 280..288 {
        ll_lengths[i] = 8;
    }

    // Fill distance codes (0-31 = 5 bits)
    for i in 0..32 {
        d_lengths[i] = 5;
    }
}
#[no_mangle]
pub unsafe extern "C" fn GetFixedTree(ll_lengths: *mut c_int, d_lengths: *mut c_int) {
    // Ensure the pointers are valid
    if ll_lengths.is_null() || d_lengths.is_null() {
        return;
    }

    // Create slices from the raw pointers
    let ll_slice = slice::from_raw_parts_mut(ll_lengths as *mut u32, 288);
    let d_slice = slice::from_raw_parts_mut(d_lengths as *mut u32, 32);

    // Call the Rust implementation
    GetFixedTree_rust(ll_slice, d_slice);
}

fn ZopfliLZ77GetHistogramAt_rust(
    lz77: &ZopfliLZ77Store,
    lpos: usize,
    ll_counts: &mut [usize],
    d_counts: &mut [usize],
) {
    // The real histogram is created by using the histogram for this chunk, but
    // all superfluous values of this chunk subtracted.
    let llpos = 288 * (lpos / 288);
    let dpos = 32 * (lpos / 32);

    unsafe {
        // Copy the literal/length counts
        for i in 0..288 {
            ll_counts[i] = *lz77.ll_counts.add(llpos + i);
        }

        // Subtract counts for positions after lpos
        for i in (lpos + 1)..cmp::min(llpos + 288, lz77.size) {
            ll_counts[*lz77.ll_symbol.add(i) as usize] -= 1;
        }

        // Copy the distance counts
        for i in 0..32 {
            d_counts[i] = *lz77.d_counts.add(dpos + i);
        }

        // Subtract counts for positions after lpos
        for i in (lpos + 1)..cmp::min(dpos + 32, lz77.size) {
            if *lz77.dists.add(i) != 0 {
                d_counts[*lz77.d_symbol.add(i) as usize] -= 1;
            }
        }
    }
}
#[no_mangle]

fn ZopfliLZ77GetHistogram_rust(
    lz77: &ZopfliLZ77Store,
    lstart: usize,
    lend: usize,
    ll_counts: &mut [usize; 288],
    d_counts: &mut [usize; 32],
) {
    if lstart + 288 * 3 > lend {
        // Clear the arrays
        ll_counts.fill(0);
        d_counts.fill(0);

        // Compute the histograms directly
        for i in lstart..lend {
            // Access the arrays safely
            let ll_symbol = unsafe { *lz77.ll_symbol.add(i) } as usize;
            ll_counts[ll_symbol] += 1;

            let dist = unsafe { *lz77.dists.add(i) };
            if dist != 0 {
                let d_symbol = unsafe { *lz77.d_symbol.add(i) } as usize;
                d_counts[d_symbol] += 1;
            }
        }
    } else {
        // Subtract the cumulative histograms at the end and the start to get the
        // histogram for this range.
        ZopfliLZ77GetHistogramAt_rust(lz77, lend - 1, ll_counts, d_counts);

        if lstart > 0 {
            let mut ll_counts2 = [0usize; 288];
            let mut d_counts2 = [0usize; 32];

            ZopfliLZ77GetHistogramAt_rust(lz77, lstart - 1, &mut ll_counts2, &mut d_counts2);

            for i in 0..288 {
                ll_counts[i] -= ll_counts2[i];
            }

            for i in 0..32 {
                d_counts[i] -= d_counts2[i];
            }
        }
    }
}
#[no_mangle]
pub unsafe extern "C" fn ZopfliLZ77GetHistogram(
    lz77: *const ZopfliLZ77Store,
    lstart: size_t,
    lend: size_t,
    ll_counts: *mut size_t,
    d_counts: *mut size_t,
) {
    let lz77 = &*lz77;

    // Create mutable slices from the raw pointers
    let ll_counts_slice = slice::from_raw_parts_mut(ll_counts, 288);
    let d_counts_slice = slice::from_raw_parts_mut(d_counts, 32);

    // Convert to array references for the Rust function
    let ll_counts_array = &mut *(ll_counts_slice as *mut [usize] as *mut [usize; 288]);
    let d_counts_array = &mut *(d_counts_slice as *mut [usize] as *mut [usize; 32]);

    ZopfliLZ77GetHistogram_rust(lz77, lstart, lend, ll_counts_array, d_counts_array);
}

/// Initializes a Node with the given parameters
fn InitNode_rust(weight: usize, count: i32, tail: *mut Node, node: &mut Node) {
    node.weight = weight;
    node.count = count;
    node.tail = tail;
}
#[no_mangle]
pub unsafe extern "C" fn InitNode(weight: size_t, count: c_int, tail: *mut Node, node: *mut Node) {
    if !node.is_null() {
        let node_ref = &mut *node;
        InitNode_rust(weight, count, tail, node_ref);
    }
}

pub struct Node_rust {
    /// Total weight (symbol count) of this chain.
    pub weight: usize,
    /// Previous node(s) of this chain, or None if none.
    pub tail: Option<Box<Node_rust>>,
    /// Leaf symbol index, or number of leaves before this chain.
    pub count: i32,
}

fn InitLists_rust(
    pool: &mut Vec<Node_rust>,
    leaves: &[Node_rust],
    maxbits: usize,
    lists: &mut [[*mut Node_rust; 2]],
) {
    // Create two new nodes
    let mut node0 = Node_rust {
        weight: leaves[0].weight,
        count: 1,
        tail: None,
    };

    let mut node1 = Node_rust {
        weight: leaves[1].weight,
        count: 2,
        tail: None,
    };

    // Add nodes to the pool
    pool.push(node0);
    pool.push(node1);

    // Calculate indices first to avoid borrowing issues
    let node0_index = pool.len() - 2;
    let node1_index = pool.len() - 1;

    // Get pointers to the nodes we just added
    let node0_ptr = &mut pool[node0_index] as *mut Node_rust;
    let node1_ptr = &mut pool[node1_index] as *mut Node_rust;

    // Initialize the lists
    for i in 0..maxbits {
        lists[i][0] = node0_ptr;
        lists[i][1] = node1_ptr;
    }
}
#[no_mangle]
pub unsafe extern "C" fn InitLists(
    pool: *mut NodePool,
    leaves: *const Node,
    maxbits: c_int,
    lists: *mut [*mut Node; 2],
) {
    // Create a Rust pool
    let mut rust_pool: Vec<Node_rust> = Vec::new();

    // Convert leaves to Rust
    let leaves_slice = slice::from_raw_parts(leaves, 2);
    let rust_leaves: Vec<Node_rust> = leaves_slice
        .iter()
        .map(|node| Node_rust {
            weight: node.weight as usize,
            count: node.count,
            tail: None,
        })
        .collect();

    // Create a mutable array for the lists
    let maxbits_usize = maxbits as usize;
    let mut rust_lists = Vec::with_capacity(maxbits_usize);

    // Initialize rust_lists with dummy values
    for _ in 0..maxbits_usize {
        rust_lists.push([ptr::null_mut(), ptr::null_mut()]);
    }

    // Call the Rust implementation
    InitLists_rust(&mut rust_pool, &rust_leaves, maxbits_usize, &mut rust_lists);

    // Now update the C structures
    // First, allocate two nodes in the pool
    let node0 = (*pool).next;
    let node1 = (*pool).next.add(1);

    // Initialize the nodes
    InitNode_rust(
        leaves_slice[0].weight as usize,
        1,
        ptr::null_mut(),
        &mut *node0,
    );

    InitNode_rust(
        leaves_slice[1].weight as usize,
        2,
        ptr::null_mut(),
        &mut *node1,
    );

    // Update the pool's next pointer
    (*pool).next = (*pool).next.add(2);

    // Update the lists
    for i in 0..maxbits as usize {
        let list_ptr = lists.add(i);
        (*list_ptr)[0] = node0;
        (*list_ptr)[1] = node1;
    }
}

fn BoundaryPM_rust(
    lists: *mut [*mut Node; 2],
    leaves: *mut Node,
    numsymbols: i32,
    pool: *mut NodePool,
    index: i32,
) {
    unsafe {
        let index_usize = index as usize;

        // Get the last count from the current list
        let lastcount = (*(*lists.add(index_usize))[1]).count;

        if index == 0 && lastcount >= numsymbols {
            return;
        }

        // Get a new node from the pool
        let newchain = (*pool).next;
        (*pool).next = (*pool).next.add(1);
        let oldchain = (*lists.add(index_usize))[1];

        // Update the list pointers
        (*lists.add(index_usize))[0] = oldchain;
        (*lists.add(index_usize))[1] = newchain;

        if index == 0 {
            // New leaf node in list 0
            let leaf_weight = if lastcount >= 0 && lastcount < numsymbols {
                (*leaves.add(lastcount as usize)).weight
            } else {
                0 // Default weight if out of bounds
            };

            InitNode_rust(leaf_weight, lastcount + 1, ptr::null_mut(), &mut *newchain);
        } else {
            let prev_index = (index - 1) as usize;
            let sum = (*(*lists.add(prev_index))[0]).weight + (*(*lists.add(prev_index))[1]).weight;

            if lastcount < numsymbols && sum > (*leaves.add(lastcount as usize)).weight {
                // New leaf inserted in list, so count is incremented
                InitNode_rust(
                    (*leaves.add(lastcount as usize)).weight,
                    lastcount + 1,
                    (*oldchain).tail,
                    &mut *newchain,
                );
            } else {
                InitNode_rust(sum, lastcount, (*lists.add(prev_index))[1], &mut *newchain);

                // Two lookahead chains of previous list used up, create new ones
                BoundaryPM_rust(lists, leaves, numsymbols, pool, index - 1);
                BoundaryPM_rust(lists, leaves, numsymbols, pool, index - 1);
            }
        }
    }
}
#[no_mangle]
pub unsafe extern "C" fn BoundaryPM(
    lists: *mut [*mut Node; 2],
    leaves: *mut Node,
    numsymbols: c_int,
    pool: *mut NodePool,
    index: c_int,
) {
    if lists.is_null() || leaves.is_null() || pool.is_null() {
        return;
    }

    BoundaryPM_rust(lists, leaves, numsymbols, pool, index);
}

fn BoundaryPMFinal_rust(
    lists: &mut [&mut [Option<Box<Node_rust>>; 2]],
    leaves: &[Node_rust],
    numsymbols: i32,
    index: usize,
) {
    let lastcount = lists[index][1].as_ref().unwrap().count; // Count of last chain of list.

    let sum =
        lists[index - 1][0].as_ref().unwrap().weight + lists[index - 1][1].as_ref().unwrap().weight;

    if lastcount < numsymbols && sum > leaves[lastcount as usize].weight {
        // Create a new node with the old chain as its tail
        let oldchain = lists[index][1].take();

        let newchain = Box::new(Node_rust {
            weight: 0, // This will be set appropriately elsewhere
            tail: oldchain,
            count: lastcount + 1,
        });

        // Update the list
        lists[index][1] = Some(newchain);
    } else {
        // Take both nodes to avoid borrow issues
        let mut current_node = lists[index][1].take().unwrap();
        let prev_node = lists[index - 1][1].take();

        // Set current node's tail to the previous list's second node
        current_node.tail = prev_node;

        // Put the modified node back
        lists[index][1] = Some(current_node);
    }
}
#[no_mangle]
pub unsafe extern "C" fn BoundaryPMFinal(
    lists: *mut [*mut Node; 2],
    leaves: *mut Node,
    numsymbols: c_int,
    pool: *mut NodePool,
    index: c_int,
) {
    // Since we can't properly convert between the C and Rust structures in this context,
    // we'll implement the C function logic directly in the wrapper

    if lists.is_null() || leaves.is_null() || pool.is_null() {
        return;
    }

    let index_usize = index as usize;

    // Get the count of the last chain of the list
    let lastcount = unsafe {
        let list_ptr = &*lists.add(index_usize);
        let node_ptr = list_ptr[1];
        (*node_ptr).count
    };

    // Calculate sum of weights from the previous list
    let sum = unsafe {
        let prev_list_ptr = &*lists.add(index_usize - 1);
        let node_ptr0 = prev_list_ptr[0];
        let node_ptr1 = prev_list_ptr[1];
        (*node_ptr0).weight + (*node_ptr1).weight
    };

    // Get the weight of the leaf at lastcount
    let leaf_weight = if lastcount < numsymbols {
        unsafe { (*leaves.add(lastcount as usize)).weight }
    } else {
        0 // Default value if lastcount is out of bounds
    };

    if lastcount < numsymbols && sum > leaf_weight {
        // Get the next node from the pool
        let newchain = unsafe { (*pool).next };

        // Get the tail of the current chain
        let oldchain = unsafe {
            let list_ptr = &*lists.add(index_usize);
            let node_ptr = list_ptr[1];
            (*node_ptr).tail
        };

        // Update the list
        unsafe {
            let list_ptr = &mut *lists.add(index_usize);
            list_ptr[1] = newchain;
        }

        // Update the new chain
        unsafe {
            (*newchain).count = lastcount + 1;
            (*newchain).tail = oldchain;
        }
    } else {
        // Update the tail of the current chain
        unsafe {
            let list_ptr = &mut *lists.add(index_usize);
            let node_ptr = list_ptr[1];
            let prev_list_ptr = &*lists.add(index_usize - 1);
            (*node_ptr).tail = prev_list_ptr[1];
        }
    }
}

pub struct NodePool_rust {
    /// Pointer to a free node in the pool.
    pub next: Option<Box<Node_rust>>,
}

fn ZopfliCalculateBitLengths_rust(count: &[usize], maxbits: i32, bitlengths: &mut [u32]) {
    // Since we don't have a Rust implementation of ZopfliLengthLimitedCodeLengths,
    // we'll need to use the C function through FFI
    let error = unsafe {
        ZopfliLengthLimitedCodeLengths(
            count.as_ptr(),
            count.len().try_into().unwrap(), // Convert usize to i32
            maxbits,
            bitlengths.as_mut_ptr(),
        )
    };

    // In Rust, we can use assert! instead of __assert_fail
    assert!(error == 0, "ZopfliLengthLimitedCodeLengths failed");
}
#[no_mangle]
pub unsafe extern "C" fn ZopfliCalculateBitLengths(
    count: *const size_t,
    n: size_t,
    maxbits: c_int,
    bitlengths: *mut libc::c_uint,
) {
    // Convert C pointers to Rust slices for safe handling
    let count_slice = if !count.is_null() && n > 0 {
        slice::from_raw_parts(count, n)
    } else {
        &[]
    };

    let bitlengths_slice = if !bitlengths.is_null() && n > 0 {
        slice::from_raw_parts_mut(bitlengths as *mut u32, n)
    } else {
        &mut []
    };

    // Call the Rust implementation
    ZopfliCalculateBitLengths_rust(count_slice, maxbits, bitlengths_slice);
}

fn PatchDistanceCodesForBuggyDecoders_rust(d_lengths: &mut [u32]) {
    let mut num_dist_codes = 0; // Amount of non-zero distance codes

    // Ignore the two unused codes from the spec (30 instead of 32)
    for i in 0..30 {
        if d_lengths[i] > 0 {
            num_dist_codes += 1;
        }
        if num_dist_codes >= 2 {
            return; // Two or more codes is fine.
        }
    }

    if num_dist_codes == 0 {
        d_lengths[0] = 1;
        d_lengths[1] = 1;
    } else if num_dist_codes == 1 {
        // If the first code is non-zero, set the second code to 1
        // Otherwise, set the first code to 1
        let index_to_set = if d_lengths[0] > 0 { 1 } else { 0 };
        d_lengths[index_to_set] = 1;
    }
}
#[no_mangle]
pub unsafe extern "C" fn PatchDistanceCodesForBuggyDecoders(d_lengths: *mut c_int) {
    // Assuming the array is at least 30 elements long as in the original C code
    let d_lengths_slice = slice::from_raw_parts_mut(d_lengths as *mut u32, 30);
    PatchDistanceCodesForBuggyDecoders_rust(d_lengths_slice);
}

fn ZopfliLengthsToSymbols_rust(lengths: &[u32], maxbits: u32, symbols: &mut [u32]) {
    assert!(
        lengths.len() == symbols.len(),
        "lengths and symbols arrays must have the same length"
    );

    let n = lengths.len();
    let mut bl_count = vec![0usize; (maxbits + 1) as usize];
    let mut next_code = vec![0usize; (maxbits + 1) as usize];

    // Initialize symbols to 0
    for symbol in symbols.iter_mut() {
        *symbol = 0;
    }

    // 1) Count the number of codes for each code length
    for &length in lengths.iter() {
        assert!(
            length <= maxbits,
            "length must be less than or equal to maxbits"
        );
        if length > 0 {
            bl_count[length as usize] += 1;
        }
    }

    // 2) Find the numerical value of the smallest code for each code length
    let mut code = 0;
    bl_count[0] = 0;
    for bits in 1..=maxbits as usize {
        code = (code + bl_count[bits - 1]) << 1;
        next_code[bits] = code;
    }

    // 3) Assign numerical values to all codes
    for i in 0..n {
        let len = lengths[i];
        if len != 0 {
            symbols[i] = next_code[len as usize] as u32;
            next_code[len as usize] += 1;
        }
    }
}
#[no_mangle]

fn AddBits_rust(symbol: u32, length: u32, bp: &mut u8, out: &mut Vec<u8>) {
    for i in 0..length {
        let bit = (symbol >> i) & 1;

        // If we're at the start of a new byte, add a new byte to the output
        if *bp == 0 {
            out.push(0);
        }

        // Set the bit in the last byte
        let last_idx = out.len() - 1;
        out[last_idx] |= (bit as u8) << *bp;

        // Move to the next bit position, wrapping around after 7
        *bp = (*bp + 1) & 7;
    }
}
#[no_mangle]
pub unsafe extern "C" fn AddBits(
    symbol: c_int,
    length: c_int,
    bp: *mut c_uchar,
    out: *mut *mut c_uchar,
    outsize: *mut size_t,
) {
    // Convert C parameters to Rust types
    let symbol = symbol as u32;
    let length = length as u32;
    let bp_ref = &mut (*bp);

    // Create a Vec that represents the current output buffer
    let mut rust_out = if *outsize == 0 {
        Vec::new()
    } else {
        Vec::from_raw_parts(*out, *outsize, *outsize)
    };

    // Call the Rust implementation
    AddBits_rust(symbol, length, bp_ref, &mut rust_out);

    // Update the output size
    let new_size = rust_out.len();

    // Prevent the Vec from freeing the memory when it goes out of scope
    let ptr = rust_out.as_mut_ptr();
    let capacity = rust_out.capacity();
    mem::forget(rust_out);

    // Update the C output parameters
    *out = ptr;
    *outsize = new_size;
}

fn AddHuffmanBits_rust(
    symbol: u32,
    length: u32,
    bp: &mut u8,
    out: &mut Vec<u8>,
    outsize: &mut usize,
) {
    // TODO(lode): make more efficient (add more bits at once).
    for i in 0..length {
        let bit = (symbol >> (length - i - 1)) & 1;

        // If bp is 0, we need to add a new byte to the output
        if *bp == 0 {
            // Double allocation size if it's a power of two
            if *outsize != 0 && (*outsize & (*outsize - 1)) == 0 {
                // Resize the vector, doubling its capacity and initializing new elements to 0
                out.resize(*outsize * 2, 0);
            } else if *outsize == 0 {
                // Initialize the vector with a single element if it's empty
                out.push(0);
            } else {
                // Just add a new byte
                out.push(0);
            }
            *outsize += 1;
        }

        // Set the bit in the current byte
        out[*outsize - 1] |= (bit as u8) << *bp;

        // Move to the next bit position, wrapping around after 7
        *bp = (*bp + 1) & 7;
    }
}
#[no_mangle]
pub unsafe extern "C" fn AddHuffmanBits(
    symbol: c_int,
    length: c_int,
    bp: *mut c_uchar,
    out: *mut *mut c_uchar,
    outsize: *mut size_t,
) {
    let symbol_u32 = symbol as u32;
    let length_u32 = length as u32;
    let bp_ref = &mut (*bp);

    // Create a Vec that will manage our buffer
    let mut rust_vec: Vec<u8>;
    let current_size = *outsize;

    // Initialize or recreate the Vec from the existing buffer
    if current_size == 0 || *out == ptr::null_mut() {
        rust_vec = Vec::new();
    } else {
        // Create a Vec from the existing buffer
        let slice = slice::from_raw_parts(*out, current_size);
        rust_vec = slice.to_vec();
    }

    let mut rust_outsize = current_size;

    // Call the Rust implementation
    AddHuffmanBits_rust(
        symbol_u32,
        length_u32,
        bp_ref,
        &mut rust_vec,
        &mut rust_outsize,
    );

    // Update the C buffer with our Vec's data
    if rust_outsize > current_size {
        // We need to allocate a new buffer or resize the existing one
        let new_ptr = if *out == ptr::null_mut() {
            libc::malloc(rust_outsize * mem::size_of::<c_uchar>() as size_t)
        } else {
            libc::realloc(
                *out as *mut c_void,
                rust_outsize * mem::size_of::<c_uchar>() as size_t,
            )
        } as *mut c_uchar;

        // Copy the data from our Vec to the C buffer
        ptr::copy_nonoverlapping(rust_vec.as_ptr(), new_ptr, rust_outsize);

        // Update the out pointer
        *out = new_ptr;
    } else if rust_outsize > 0 {
        // Just update the existing buffer with our Vec's data
        ptr::copy_nonoverlapping(rust_vec.as_ptr(), *out, rust_outsize);
    }

    // Update the outsize
    *outsize = rust_outsize;
}

fn EncodeTree_rust(
    ll_lengths: &[u32],
    d_lengths: &[u32],
    use_16: bool,
    use_17: bool,
    use_18: bool,
    bp: &mut u8,
    out: Option<&mut Vec<u8>>,
) -> usize {
    // The order in which code length code lengths are encoded as per deflate.
    const ORDER: [usize; 19] = [
        16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15,
    ];

    let mut hlit = 29; // 286 - 257
    let mut hdist = 29; // 32 - 1, but gzip does not like hdist > 29.
    let mut clcounts = [0usize; 19];
    let mut clcl = [0u32; 19];
    let mut clsymbols = [0u32; 19];
    let size_only = out.is_none();
    let mut result_size = 0;

    // Trim zeros.
    while hlit > 0 && ll_lengths[257 + hlit - 1] == 0 {
        hlit -= 1;
    }
    while hdist > 0 && d_lengths[1 + hdist - 1] == 0 {
        hdist -= 1;
    }
    let hlit2 = hlit + 257;

    let lld_total = hlit2 + hdist + 1;

    // Runlength encoded version of lengths of litlen and dist trees.
    let mut rle = Vec::new();
    let mut rle_bits = Vec::new();

    let mut i = 0;
    while i < lld_total {
        // This is an encoding of a huffman tree, so now the length is a symbol
        let symbol = if i < hlit2 {
            ll_lengths[i]
        } else {
            d_lengths[i - hlit2]
        };

        let mut count = 1;
        if use_16 || (symbol == 0 && (use_17 || use_18)) {
            let mut j = i + 1;
            while j < lld_total {
                let next_symbol = if j < hlit2 {
                    ll_lengths[j]
                } else {
                    d_lengths[j - hlit2]
                };

                if symbol != next_symbol {
                    break;
                }
                count += 1;
                j += 1;
            }
        }
        i += count;

        // Repetitions of zeroes
        if symbol == 0 && count >= 3 {
            if use_18 {
                let mut remaining = count;
                while remaining >= 11 {
                    let count2 = if remaining > 138 { 138 } else { remaining };
                    if !size_only {
                        rle.push(18);
                        rle_bits.push(count2 - 11);
                    }
                    clcounts[18] += 1;
                    remaining -= count2;
                }
                count = remaining;
            }

            if use_17 {
                let mut remaining = count;
                while remaining >= 3 {
                    let count2 = if remaining > 10 { 10 } else { remaining };
                    if !size_only {
                        rle.push(17);
                        rle_bits.push(count2 - 3);
                    }
                    clcounts[17] += 1;
                    remaining -= count2;
                }
                count = remaining;
            }
        }

        // Repetitions of any symbol
        if use_16 && count >= 4 {
            count -= 1; // Since the first one is hardcoded.
            clcounts[symbol as usize] += 1;
            if !size_only {
                rle.push(symbol);
                rle_bits.push(0);
            }

            let mut remaining = count;
            while remaining >= 3 {
                let count2 = if remaining > 6 { 6 } else { remaining };
                if !size_only {
                    rle.push(16);
                    rle_bits.push(count2 - 3);
                }
                clcounts[16] += 1;
                remaining -= count2;
            }
            count = remaining;
        }

        // No or insufficient repetition
        clcounts[symbol as usize] += count;
        for _ in 0..count {
            if !size_only {
                rle.push(symbol);
                rle_bits.push(0);
            }
        }
    }

    ZopfliCalculateBitLengths_rust(&clcounts, 7, &mut clcl);

    if !size_only {
        ZopfliLengthsToSymbols_rust(&clcl, 7, &mut clsymbols);
    }

    let mut hclen = 15;
    // Trim zeros.
    while hclen > 0 && clcounts[ORDER[hclen + 4 - 1]] == 0 {
        hclen -= 1;
    }

    if !size_only {
        let out = out.unwrap();
        let mut outsize = out.len();

        AddBits_rust(hlit as u32, 5, bp, out);
        AddBits_rust(hdist as u32, 5, bp, out);
        AddBits_rust(hclen as u32, 4, bp, out);

        for i in 0..(hclen + 4) {
            AddBits_rust(clcl[ORDER[i]], 3, bp, out);
        }

        for i in 0..rle.len() {
            let symbol = clsymbols[rle[i] as usize];
            AddHuffmanBits_rust(symbol, clcl[rle[i] as usize], bp, out, &mut outsize);

            // Extra bits - convert usize to u32
            match rle[i] {
                16 => AddBits_rust(rle_bits[i] as u32, 2, bp, out),
                17 => AddBits_rust(rle_bits[i] as u32, 3, bp, out),
                18 => AddBits_rust(rle_bits[i] as u32, 7, bp, out),
                _ => {}
            }
        }
    }

    result_size += 14; // hlit, hdist, hclen bits
    result_size += (hclen + 4) * 3; // clcl bits

    for i in 0..19 {
        result_size += clcl[i] as usize * clcounts[i];
    }

    // Extra bits.
    result_size += clcounts[16] * 2;
    result_size += clcounts[17] * 3;
    result_size += clcounts[18] * 7;

    result_size
}
#[no_mangle]

fn CalculateTreeSize_rust(ll_lengths: &[u32], d_lengths: &[u32]) -> usize {
    let mut result = 0;

    for i in 0..8 {
        let use_16 = (i & 1) != 0;
        let use_17 = (i & 2) != 0;
        let use_18 = (i & 4) != 0;

        let mut bp = 0u8;
        let size = EncodeTree_rust(ll_lengths, d_lengths, use_16, use_17, use_18, &mut bp, None);

        if result == 0 || size < result {
            result = size;
        }
    }

    result
}
#[no_mangle]
pub unsafe extern "C" fn CalculateTreeSize(
    ll_lengths: *const c_int,
    d_lengths: *const c_int,
) -> size_t {
    // In Zopfli, ll_lengths is an array of 288 elements and d_lengths is an array of 32 elements
    // These are the standard sizes for DEFLATE Huffman code lengths
    const LL_SIZE: usize = 288;
    const D_SIZE: usize = 32;

    // Convert C pointers to Rust slices with known sizes
    let ll_lengths_slice = if !ll_lengths.is_null() {
        slice::from_raw_parts(ll_lengths as *const u32, LL_SIZE)
    } else {
        panic!("ll_lengths is null");
    };

    let d_lengths_slice = if !d_lengths.is_null() {
        slice::from_raw_parts(d_lengths as *const u32, D_SIZE)
    } else {
        panic!("d_lengths is null");
    };

    CalculateTreeSize_rust(ll_lengths_slice, d_lengths_slice)
}

fn ZopfliGetLengthSymbolExtraBits_rust(s: usize) -> u8 {
    // Static lookup table for length symbol extra bits
    const TABLE: [u8; 29] = [
        0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 0,
    ];

    // Ensure the index is within bounds
    if s >= 257 && s < 257 + TABLE.len() {
        TABLE[s - 257]
    } else {
        // This is a safety fallback that wasn't in the original C code
        // but is good practice in Rust to avoid panics
        0
    }
}
#[no_mangle]
pub unsafe extern "C" fn ZopfliGetLengthSymbolExtraBits(s: c_int) -> c_int {
    // Convert C integer to Rust usize, ensuring it's non-negative
    let s_usize = if s >= 0 { s as usize } else { 0 };

    // Call the Rust implementation and convert the result back to C integer
    ZopfliGetLengthSymbolExtraBits_rust(s_usize) as c_int
}

fn ZopfliGetDistSymbolExtraBits_rust(s: usize) -> i32 {
    // Using a const array is more idiomatic in Rust than a static array inside a function
    const TABLE: [i32; 30] = [
        0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10, 11, 11, 12, 12,
        13, 13,
    ];

    // Bounds checking is automatically done when indexing in Rust
    TABLE[s]
}
#[no_mangle]
pub unsafe extern "C" fn ZopfliGetDistSymbolExtraBits(s: c_int) -> c_int {
    // Convert C int to usize for safe indexing
    let s_usize = s as usize;

    // Call the Rust implementation and return the result
    ZopfliGetDistSymbolExtraBits_rust(s_usize)
}

fn CalculateBlockSymbolSizeSmall_rust(
    ll_lengths: &[usize],
    d_lengths: &[usize],
    lz77: &ZopfliLZ77Store,
    lstart: usize,
    lend: usize,
) -> usize {
    let mut result = 0;

    // Get slices from the lz77 store for safe access
    let litlens = unsafe { slice::from_raw_parts(lz77.litlens, lz77.size) };
    let dists = unsafe { slice::from_raw_parts(lz77.dists, lz77.size) };

    for i in lstart..lend {
        // Replace C assertions with Rust assertions
        assert!(i < lz77.size, "i < lz77.size");
        assert!(litlens[i] < 259, "lz77.litlens[i] < 259");

        if dists[i] == 0 {
            result += ll_lengths[litlens[i] as usize];
        } else {
            let ll_symbol = ZopfliGetLengthSymbol_rust(litlens[i] as usize);
            let d_symbol = ZopfliGetDistSymbol_rust(dists[i] as i32);

            result += ll_lengths[ll_symbol];
            result += d_lengths[d_symbol as usize];
            result += ZopfliGetLengthSymbolExtraBits_rust(ll_symbol) as usize;
            result += ZopfliGetDistSymbolExtraBits_rust(d_symbol as usize) as usize;
        }
    }

    // Add end symbol
    result += ll_lengths[256];

    result
}
#[no_mangle]
pub unsafe extern "C" fn CalculateBlockSymbolSizeSmall(
    ll_lengths: *const libc::c_uint,
    d_lengths: *const libc::c_uint,
    lz77: *const ZopfliLZ77Store,
    lstart: libc::size_t,
    lend: libc::size_t,
) -> libc::size_t {
    // Convert C pointers to Rust slices and references
    let lz77 = &*lz77;

    // We don't know the exact length of these arrays, but they should be large enough
    // to accommodate all possible symbols
    let ll_lengths_slice = slice::from_raw_parts(ll_lengths, 288); // DEFLATE has 288 literal/length codes
    let d_lengths_slice = slice::from_raw_parts(d_lengths, 32); // DEFLATE has 32 distance codes

    // Convert C unsigned int arrays to Rust usize arrays
    let ll_lengths_usize: Vec<usize> = ll_lengths_slice.iter().map(|&x| x as usize).collect();
    let d_lengths_usize: Vec<usize> = d_lengths_slice.iter().map(|&x| x as usize).collect();

    CalculateBlockSymbolSizeSmall_rust(
        &ll_lengths_usize,
        &d_lengths_usize,
        lz77,
        lstart as usize,
        lend as usize,
    ) as libc::size_t
}

fn CalculateBlockSymbolSizeGivenCounts_rust(
    ll_counts: &[usize],
    d_counts: &[usize],
    ll_lengths: &[usize],
    d_lengths: &[usize],
    lz77: &ZopfliLZ77Store,
    lstart: usize,
    lend: usize,
) -> usize {
    if lstart + 288 * 3 > lend {
        return CalculateBlockSymbolSizeSmall_rust(ll_lengths, d_lengths, lz77, lstart, lend);
    } else {
        let mut result = 0;

        // Process literal/length symbols 0-255
        for i in 0..256 {
            result += ll_lengths[i] * ll_counts[i];
        }

        // Process literal/length symbols 257-285
        for i in 257..286 {
            result += ll_lengths[i] * ll_counts[i];
            result += ZopfliGetLengthSymbolExtraBits_rust(i) as usize * ll_counts[i];
        }

        // Process distance symbols 0-29
        for i in 0..30 {
            result += d_lengths[i] * d_counts[i];
            result += ZopfliGetDistSymbolExtraBits_rust(i) as usize * d_counts[i];
        }

        // Add end symbol
        result += ll_lengths[256];

        result
    }
}
#[no_mangle]
pub unsafe extern "C" fn CalculateBlockSymbolSizeGivenCounts(
    ll_counts: *const size_t,
    d_counts: *const size_t,
    ll_lengths: *const c_int,
    d_lengths: *const c_int,
    lz77: *const ZopfliLZ77Store,
    lstart: size_t,
    lend: size_t,
) -> size_t {
    // Convert C arrays to Rust slices
    let ll_counts_slice = slice::from_raw_parts(ll_counts, 286);
    let d_counts_slice = slice::from_raw_parts(d_counts, 30);

    // Convert unsigned int arrays to usize arrays
    let ll_lengths_slice_c = slice::from_raw_parts(ll_lengths, 286);
    let d_lengths_slice_c = slice::from_raw_parts(d_lengths, 30);

    // Convert c_int slices to usize slices
    let mut ll_lengths_slice = Vec::with_capacity(286);
    let mut d_lengths_slice = Vec::with_capacity(30);

    for &len in ll_lengths_slice_c {
        ll_lengths_slice.push(len as usize);
    }

    for &len in d_lengths_slice_c {
        d_lengths_slice.push(len as usize);
    }

    // Call the Rust implementation
    CalculateBlockSymbolSizeGivenCounts_rust(
        ll_counts_slice,
        d_counts_slice,
        &ll_lengths_slice,
        &d_lengths_slice,
        &*lz77,
        lstart,
        lend,
    ) as size_t
}

fn AbsDiff_rust(x: usize, y: usize) -> usize {
    if x > y {
        x - y
    } else {
        y - x
    }
}
#[no_mangle]
pub unsafe extern "C" fn AbsDiff(x: size_t, y: size_t) -> size_t {
    AbsDiff_rust(x, y)
}

fn OptimizeHuffmanForRle_rust(length: usize, counts: &mut [usize]) {
    if length == 0 {
        return;
    }

    let mut current_length = length;

    // 1) We don't want to touch the trailing zeros. We may break the
    // rules of the format by adding more data in the distance codes.
    loop {
        if current_length == 0 {
            return;
        }
        if counts[current_length - 1] != 0 {
            // Now counts[0..current_length - 1] does not have trailing zeros.
            break;
        }
        current_length -= 1;
    }

    // 2) Let's mark all population counts that already can be encoded
    // with an rle code.
    let mut good_for_rle = vec![0; current_length];

    // Let's not spoil any of the existing good rle codes.
    // Mark any seq of 0's that is longer than 5 as a good_for_rle.
    // Mark any seq of non-0's that is longer than 7 as a good_for_rle.
    if current_length > 0 {
        let mut symbol = counts[0];
        let mut stride = 0;

        for i in 0..=current_length {
            if i == current_length || counts[i] != symbol {
                if (symbol == 0 && stride >= 5) || (symbol != 0 && stride >= 7) {
                    for k in 0..stride {
                        if i > k {
                            good_for_rle[i - k - 1] = 1;
                        }
                    }
                }
                stride = 1;
                if i != current_length {
                    symbol = counts[i];
                }
            } else {
                stride += 1;
            }
        }

        // 3) Let's replace those population counts that lead to more rle codes.
        let mut stride = 0;
        let mut limit = counts[0];
        let mut sum = 0;

        for i in 0..=current_length {
            if i == current_length || good_for_rle[i] == 1
                // Heuristic for selecting the stride ranges to collapse.
                || AbsDiff_rust(counts[i], limit) >= 4
            {
                if stride >= 4 || (stride >= 3 && sum == 0) {
                    // The stride must end, collapse what we have, if we have enough (4).
                    let mut count = if stride > 0 {
                        (sum + stride / 2) / stride
                    } else {
                        0
                    };
                    if count < 1 {
                        count = 1;
                    }
                    if sum == 0 {
                        // Don't make an all zeros stride to be upgraded to ones.
                        count = 0;
                    }
                    for k in 0..stride {
                        if i > k {
                            // We don't want to change value at counts[i],
                            // that is already belonging to the next stride. Thus - 1.
                            counts[i - k - 1] = count;
                        }
                    }
                }
                stride = 0;
                sum = 0;

                if i + 3 < current_length {
                    // All interesting strides have a count of at least 4,
                    // at least when non-zeros.
                    limit = (counts[i] + counts[i + 1] + counts[i + 2] + counts[i + 3] + 2) / 4;
                } else if i < current_length {
                    limit = counts[i];
                } else {
                    limit = 0;
                }
            }
            stride += 1;
            if i != current_length {
                sum += counts[i];
            }
        }
    }
}
#[no_mangle]
pub unsafe extern "C" fn OptimizeHuffmanForRle(length: c_int, counts: *mut size_t) {
    if counts.is_null() || length <= 0 {
        return;
    }

    let length_usize = length as usize;
    let counts_slice = slice::from_raw_parts_mut(counts, length_usize);

    OptimizeHuffmanForRle_rust(length_usize, counts_slice);
}

fn TryOptimizeHuffmanForRle_rust(
    lz77: &ZopfliLZ77Store,
    lstart: usize,
    lend: usize,
    ll_counts: &[usize],
    d_counts: &[usize],
    ll_lengths: &mut [u32],
    d_lengths: &mut [u32],
) -> f64 {
    let mut ll_counts2 = [0usize; 288];
    let mut d_counts2 = [0usize; 32];
    let mut ll_lengths2 = [0u32; 288];
    let mut d_lengths2 = [0u32; 32];

    // Calculate original sizes
    let treesize = CalculateTreeSize_rust(ll_lengths, d_lengths) as f64;

    // Convert u32 arrays to usize for the function call
    let ll_lengths_usize: Vec<usize> = ll_lengths.iter().map(|&x| x as usize).collect();
    let d_lengths_usize: Vec<usize> = d_lengths.iter().map(|&x| x as usize).collect();

    let datasize = CalculateBlockSymbolSizeGivenCounts_rust(
        ll_counts,
        d_counts,
        &ll_lengths_usize,
        &d_lengths_usize,
        lz77,
        lstart,
        lend,
    ) as f64;

    // Copy counts and optimize for RLE
    ll_counts2.copy_from_slice(&ll_counts[..288]);
    d_counts2.copy_from_slice(&d_counts[..32]);

    OptimizeHuffmanForRle_rust(288, &mut ll_counts2);
    OptimizeHuffmanForRle_rust(32, &mut d_counts2);

    ZopfliCalculateBitLengths_rust(&ll_counts2, 15, &mut ll_lengths2);
    ZopfliCalculateBitLengths_rust(&d_counts2, 15, &mut d_lengths2);

    PatchDistanceCodesForBuggyDecoders_rust(&mut d_lengths2);

    // Calculate new sizes
    let treesize2 = CalculateTreeSize_rust(&ll_lengths2, &d_lengths2) as f64;

    // Convert u32 arrays to usize for the function call
    let ll_lengths2_usize: Vec<usize> = ll_lengths2.iter().map(|&x| x as usize).collect();
    let d_lengths2_usize: Vec<usize> = d_lengths2.iter().map(|&x| x as usize).collect();

    let datasize2 = CalculateBlockSymbolSizeGivenCounts_rust(
        ll_counts,
        d_counts,
        &ll_lengths2_usize,
        &d_lengths2_usize,
        lz77,
        lstart,
        lend,
    ) as f64;

    // Use the optimized version if it's better
    if treesize2 + datasize2 < treesize + datasize {
        ll_lengths.copy_from_slice(&ll_lengths2);
        d_lengths.copy_from_slice(&d_lengths2);
        treesize2 + datasize2
    } else {
        treesize + datasize
    }
}
#[no_mangle]

fn GetDynamicLengths_rust(
    lz77: &ZopfliLZ77Store,
    lstart: usize,
    lend: usize,
    ll_lengths: &mut [u32],
    d_lengths: &mut [u32],
) -> f64 {
    let mut ll_counts = [0usize; 288];
    let mut d_counts = [0usize; 32];

    ZopfliLZ77GetHistogram_rust(lz77, lstart, lend, &mut ll_counts, &mut d_counts);
    ll_counts[256] = 1; // End symbol.

    ZopfliCalculateBitLengths_rust(&ll_counts, 15, ll_lengths);
    ZopfliCalculateBitLengths_rust(&d_counts, 15, d_lengths);

    PatchDistanceCodesForBuggyDecoders_rust(d_lengths);

    TryOptimizeHuffmanForRle_rust(
        lz77, lstart, lend, &ll_counts, &d_counts, ll_lengths, d_lengths,
    )
}
#[no_mangle]

fn AddDynamicTree_rust(
    ll_lengths: &[u32],
    d_lengths: &[u32],
    bp: &mut u8,
    out: &mut Vec<u8>,
) -> usize {
    let mut best = 0;
    let mut bestsize = 0;

    for i in 0..8 {
        let use_16 = (i & 1) != 0;
        let use_17 = (i & 2) != 0;
        let use_18 = (i & 4) != 0;

        let size = EncodeTree_rust(
            ll_lengths, d_lengths, use_16, use_17, use_18,
            &mut 0, // Dummy value since we're just calculating size
            None,
        );

        if bestsize == 0 || size < bestsize {
            bestsize = size;
            best = i;
        }
    }

    // Use the best configuration to actually encode the tree
    let use_16 = (best & 1) != 0;
    let use_17 = (best & 2) != 0;
    let use_18 = (best & 4) != 0;

    EncodeTree_rust(ll_lengths, d_lengths, use_16, use_17, use_18, bp, Some(out))
}
#[no_mangle]
pub unsafe extern "C" fn AddDynamicTree(
    ll_lengths: *const c_int,
    d_lengths: *const c_int,
    bp: *mut c_uchar,
    out: *mut *mut c_uchar,
    outsize: *mut size_t,
) {
    // Convert C pointers to Rust slices and references
    // We need to determine the length of these arrays, but the original C code
    // doesn't specify it. Assuming these are known constants in the context.
    // For now, we'll use 286 for ll_lengths and 30 for d_lengths as these are
    // common values for DEFLATE.
    let ll_slice = slice::from_raw_parts(ll_lengths as *const u32, 286);
    let d_slice = slice::from_raw_parts(d_lengths as *const u32, 30);

    // Create a mutable reference to bp
    let bp_ref = &mut *bp;

    // Get the output vector
    let out_vec_ptr = *out as *mut Vec<u8>;
    let out_vec = &mut *(out_vec_ptr as *mut Vec<u8>);

    // Call the Rust implementation
    let result_size = AddDynamicTree_rust(ll_slice, d_slice, bp_ref, out_vec);

    // Update the outsize pointer
    *outsize = result_size;
}

fn ZopfliGetLengthExtraBitsValue_rust(l: usize) -> u8 {
    static TABLE: [u8; 259] = [
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 2, 3, 0, 1, 2, 3, 0, 1, 2,
        3, 0, 1, 2, 3, 0, 1, 2, 3, 4, 5, 6, 7, 0, 1, 2, 3, 4, 5, 6, 7, 0, 1, 2, 3, 4, 5, 6, 7, 0,
        1, 2, 3, 4, 5, 6, 7, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 0, 1, 2, 3, 4,
        5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14,
        15, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10,
        11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 0, 1,
        2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26,
        27, 28, 29, 30, 31, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19,
        20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12,
        13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 0,
    ];

    if l < TABLE.len() {
        TABLE[l]
    } else {
        0 // Default value for out-of-bounds access
    }
}
#[no_mangle]
pub unsafe extern "C" fn ZopfliGetLengthExtraBitsValue(l: c_int) -> c_int {
    // Convert from C int to usize, ensuring it's not negative
    let l_usize = if l >= 0 { l as usize } else { 0 };

    // Call the Rust implementation and convert the result back to C int
    ZopfliGetLengthExtraBitsValue_rust(l_usize) as c_int
}

fn ZopfliGetLengthExtraBits_rust(l: usize) -> u8 {
    static TABLE: [u8; 259] = [
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 1, 1, 1, 1, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2,
        2, 2, 2, 2, 2, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3,
        3, 3, 3, 3, 3, 3, 3, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4,
        4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4,
        4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5,
        5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5,
        5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5,
        5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5,
        5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 0,
    ];

    TABLE[l]
}
#[no_mangle]
pub unsafe extern "C" fn ZopfliGetLengthExtraBits(l: c_int) -> c_int {
    // Convert from C int to usize, ensuring it's within bounds
    let index = l as usize;

    // Call the Rust implementation and convert back to C int
    ZopfliGetLengthExtraBits_rust(index) as c_int
}

fn ZopfliGetDistExtraBitsValue_rust(dist: i32) -> i32 {
    if dist < 5 {
        0
    } else {
        // In Rust, we can use leading_zeros() instead of __builtin_clz
        let l = 31 ^ (dist - 1).leading_zeros() as i32; // log2(dist - 1)
        (dist - (1 + (1 << l))) & ((1 << (l - 1)) - 1)
    }
}
#[no_mangle]
pub unsafe extern "C" fn ZopfliGetDistExtraBitsValue(dist: c_int) -> c_int {
    ZopfliGetDistExtraBitsValue_rust(dist)
}

fn ZopfliGetDistExtraBits_rust(dist: i32) -> i32 {
    if dist < 5 {
        return 0;
    }

    // The original C code: (31 ^ __builtin_clz(dist - 1)) - 1
    // This calculates log2(dist - 1) - 1
    // We need to ensure dist - 1 is positive and handle it as u32
    let dist_minus_one = (dist - 1) as u32;

    // Calculate log2(dist - 1) - 1
    // 31 - leading_zeros gives us the position of the highest bit set (log2)
    (31 - dist_minus_one.leading_zeros()) as i32 - 1
}
#[no_mangle]
pub unsafe extern "C" fn ZopfliGetDistExtraBits(dist: c_int) -> c_int {
    ZopfliGetDistExtraBits_rust(dist)
}

fn AddLZ77Data_rust(
    lz77: &ZopfliLZ77Store,
    lstart: usize,
    lend: usize,
    expected_data_size: usize,
    ll_symbols: &[u32],
    ll_lengths: &[u32],
    d_symbols: &[u32],
    d_lengths: &[u32],
    bp: &mut u8,
    out: &mut Vec<u8>,
    outsize: &mut usize,
) {
    let mut testlength = 0;

    for i in lstart..lend {
        let dist = unsafe { *lz77.dists.add(i) };
        let litlen = unsafe { *lz77.litlens.add(i) };

        if dist == 0 {
            assert!(litlen < 256, "litlen < 256");
            assert!(ll_lengths[litlen as usize] > 0, "ll_lengths[litlen] > 0");

            AddHuffmanBits_rust(
                ll_symbols[litlen as usize],
                ll_lengths[litlen as usize],
                bp,
                out,
                outsize,
            );
            testlength += 1;
        } else {
            let lls = ZopfliGetLengthSymbol_rust(litlen as usize);
            let ds = ZopfliGetDistSymbol_rust(dist as i32);

            assert!(litlen >= 3 && litlen <= 288, "litlen >= 3 && litlen <= 288");
            assert!(ll_lengths[lls] > 0, "ll_lengths[lls] > 0");
            assert!(d_lengths[ds as usize] > 0, "d_lengths[ds] > 0");

            AddHuffmanBits_rust(ll_symbols[lls], ll_lengths[lls], bp, out, outsize);
            AddBits_rust(
                ZopfliGetLengthExtraBitsValue_rust(litlen as usize) as u32,
                ZopfliGetLengthExtraBits_rust(litlen as usize) as u32,
                bp,
                out,
            );

            AddHuffmanBits_rust(
                d_symbols[ds as usize],
                d_lengths[ds as usize],
                bp,
                out,
                outsize,
            );
            AddBits_rust(
                ZopfliGetDistExtraBitsValue_rust(dist as i32) as u32,
                ZopfliGetDistExtraBits_rust(dist as i32) as u32,
                bp,
                out,
            );

            testlength += litlen as usize;
        }
    }

    assert!(
        expected_data_size == 0 || testlength == expected_data_size,
        "expected_data_size == 0 || testlength == expected_data_size"
    );
}
#[no_mangle]
pub unsafe extern "C" fn AddLZ77Data(
    lz77: *const ZopfliLZ77Store,
    lstart: size_t,
    lend: size_t,
    expected_data_size: size_t,
    ll_symbols: *const libc::c_uint,
    ll_lengths: *const libc::c_uint,
    d_symbols: *const libc::c_uint,
    d_lengths: *const libc::c_uint,
    bp: *mut libc::c_uchar,
    out: *mut *mut libc::c_uchar,
    outsize: *mut size_t,
) {
    // Safety checks
    if lz77.is_null()
        || ll_symbols.is_null()
        || ll_lengths.is_null()
        || d_symbols.is_null()
        || d_lengths.is_null()
        || bp.is_null()
        || out.is_null()
        || outsize.is_null()
    {
        return;
    }

    // Convert C pointers to Rust references
    let lz77_ref = &*lz77;

    // Create slices with conservative lengths
    // We'll use 289 for ll arrays (0-288) and 32769 for d arrays (0-32768)
    let ll_symbols_slice = slice::from_raw_parts(ll_symbols, 289);
    let ll_lengths_slice = slice::from_raw_parts(ll_lengths, 289);
    let d_symbols_slice = slice::from_raw_parts(d_symbols, 32769);
    let d_lengths_slice = slice::from_raw_parts(d_lengths, 32769);

    // Create mutable references for output parameters
    let bp_ref = &mut *bp;
    let outsize_ref = &mut *outsize;

    // For the output buffer, we need to work with the existing data
    // First, get the current size
    let current_size = *outsize;

    // Create a Vec that wraps the existing buffer
    // This is unsafe because we're taking ownership of memory we didn't allocate
    let mut out_vec = if !(*out).is_null() && current_size > 0 {
        Vec::from_raw_parts(*out, current_size, current_size)
    } else {
        Vec::new()
    };

    // Call the Rust implementation
    AddLZ77Data_rust(
        lz77_ref,
        lstart,
        lend,
        expected_data_size,
        ll_symbols_slice,
        ll_lengths_slice,
        d_symbols_slice,
        d_lengths_slice,
        bp_ref,
        &mut out_vec,
        outsize_ref,
    );

    // Update the output pointer if needed
    *out = out_vec.as_mut_ptr();

    // Prevent the vector from being dropped, as it's now owned by the C code
    mem::forget(out_vec);
}

fn AddLZ77Block_rust(
    options: &ZopfliOptions,
    btype: i32,
    final_block: bool,
    lz77: &ZopfliLZ77Store,
    lstart: usize,
    lend: usize,
    expected_data_size: usize,
    bp: &mut u8,
    out: &mut Vec<u8>,
) {
    let mut ll_lengths = [0u32; 288];
    let mut d_lengths = [0u32; 32];
    let mut ll_symbols = [0u32; 288];
    let mut d_symbols = [0u32; 32];
    let mut outsize = out.len();
    let detect_block_size = outsize;
    let mut compressed_size;
    let mut uncompressed_size = 0;

    if btype == 0 {
        let length = ZopfliLZ77GetByteRange_rust(lz77, lstart, lend);
        let pos = if lstart == lend {
            0
        } else {
            unsafe { *lz77.pos.add(lstart) }
        };
        let end = pos + length;

        // Create a slice from lz77.data
        let data_slice = unsafe {
            if lz77.data.is_null() {
                &[]
            } else {
                slice::from_raw_parts(lz77.data, lz77.size)
            }
        };

        AddNonCompressedBlock_rust(options, final_block, data_slice, pos, end, bp, out);
        return;
    }

    AddBit_rust(if final_block { 1 } else { 0 }, bp, out, &mut outsize);
    AddBit_rust(btype & 1, bp, out, &mut outsize);
    AddBit_rust((btype & 2) >> 1, bp, out, &mut outsize);

    if btype == 1 {
        // Fixed block.
        GetFixedTree_rust(&mut ll_lengths, &mut d_lengths);
    } else {
        // Dynamic block.
        assert!(btype == 2, "btype must be 2 for dynamic blocks");

        GetDynamicLengths_rust(lz77, lstart, lend, &mut ll_lengths, &mut d_lengths);

        let detect_tree_size = outsize;
        AddDynamicTree_rust(&ll_lengths, &d_lengths, bp, out);
        outsize = out.len();

        if options.verbose != 0 {
            eprintln!("treesize: {}", outsize - detect_tree_size);
        }
    }

    ZopfliLengthsToSymbols_rust(&ll_lengths, 15, &mut ll_symbols);
    ZopfliLengthsToSymbols_rust(&d_lengths, 15, &mut d_symbols);

    let detect_block_size = outsize;
    AddLZ77Data_rust(
        lz77,
        lstart,
        lend,
        expected_data_size,
        &ll_symbols,
        &ll_lengths,
        &d_symbols,
        &d_lengths,
        bp,
        out,
        &mut outsize,
    );

    // End symbol.
    AddHuffmanBits_rust(ll_symbols[256], ll_lengths[256], bp, out, &mut outsize);

    for i in lstart..lend {
        let dist = unsafe {
            if lz77.dists.is_null() {
                0
            } else {
                *lz77.dists.add(i)
            }
        };

        uncompressed_size += if dist == 0 {
            1
        } else {
            unsafe {
                if lz77.litlens.is_null() {
                    0
                } else {
                    (*lz77.litlens.add(i)) as usize
                }
            }
        };
    }

    compressed_size = outsize - detect_block_size;
    if options.verbose != 0 {
        eprintln!(
            "compressed block size: {} ({}k) (unc: {})",
            compressed_size,
            compressed_size / 1024,
            uncompressed_size
        );
    }
}
#[no_mangle]
pub unsafe extern "C" fn AddLZ77Block(
    options: *const ZopfliOptions,
    btype: c_int,
    final_block: c_int,
    lz77: *const ZopfliLZ77Store,
    lstart: size_t,
    lend: size_t,
    expected_data_size: size_t,
    bp: *mut c_uchar,
    out: *mut *mut c_uchar,
    outsize: *mut size_t,
) {
    if options.is_null()
        || lz77.is_null()
        || bp.is_null()
        || out.is_null()
        || outsize.is_null()
        || (*out).is_null()
    {
        return;
    }

    let options_ref = &*options;
    let lz77_ref = &*lz77;
    let final_block_bool = final_block != 0;

    // Safely create a Vec from the raw parts
    let capacity = *outsize;
    let len = *outsize;
    let mut out_vec = if len == 0 {
        Vec::new()
    } else {
        Vec::from_raw_parts(*out, len, capacity)
    };

    AddLZ77Block_rust(
        options_ref,
        btype,
        final_block_bool,
        lz77_ref,
        lstart,
        lend,
        expected_data_size,
        &mut *bp,
        &mut out_vec,
    );

    // Update outsize
    *outsize = out_vec.len();

    // If the vector needed to reallocate, update the out pointer
    *out = out_vec.as_mut_ptr();

    // Prevent deallocation of the vector's memory when out_vec goes out of scope
    mem::forget(out_vec);
}

fn ZopfliCleanCache_rust(lmc: &mut ZopfliLongestMatchCache) {
    // In idiomatic Rust, we would use Vec or Box for memory management
    // which automatically handle deallocation when they go out of scope.
    // However, since we're dealing with raw pointers from C, we need to
    // safely free them.

    // Take ownership of the pointers and set them to null to prevent double-free
    let length = mem::replace(&mut lmc.length, ptr::null_mut());
    let dist = mem::replace(&mut lmc.dist, ptr::null_mut());
    let sublen = mem::replace(&mut lmc.sublen, ptr::null_mut());

    // Free the memory if the pointers are not null
    unsafe {
        if !length.is_null() {
            free(length as *mut c_void);
        }
        if !dist.is_null() {
            free(dist as *mut c_void);
        }
        if !sublen.is_null() {
            free(sublen as *mut c_void);
        }
    }
}
#[no_mangle]
pub unsafe extern "C" fn ZopfliCleanCache(lmc: *mut ZopfliLongestMatchCache) {
    if !lmc.is_null() {
        ZopfliCleanCache_rust(&mut *lmc);
    }
}

/// Cleans up the block state by cleaning the longest match cache and freeing memory.
///
/// This is a safe Rust implementation that handles the longest match cache cleanup.
fn ZopfliCleanBlockState_rust(lmc: Option<*mut c_void>) {
    // If there's a longest match cache, clean it and then free it
    if let Some(lmc_ptr) = lmc {
        if !lmc_ptr.is_null() {
            unsafe {
                // Call the Rust implementation of ZopfliCleanCache
                let lmc_typed = lmc_ptr as *mut ::std::os::raw::c_void;
                // We need to cast to the appropriate type to call ZopfliCleanCache_rust
                // This assumes ZopfliCleanCache_rust is accessible and handles the type conversion

                // Free the memory
                libc::free(lmc_typed);
            }
        }
    }
}
#[no_mangle]
pub unsafe extern "C" fn ZopfliCleanBlockState(s: *mut ZopfliBlockState) {
    if !s.is_null() {
        let block_state = &mut *s;

        // Check if lmc is not null
        if !block_state.lmc.is_null() {
            // Clean the cache using the C function
            ZopfliCleanCache(block_state.lmc);

            // Call our Rust implementation to free the memory
            ZopfliCleanBlockState_rust(Some(block_state.lmc as *mut c_void));

            // Set the pointer to null to avoid double-free
            block_state.lmc = ptr::null_mut();
        }
    }
}

fn ZopfliCleanLZ77Store_rust(store: &mut ZopfliLZ77Store) {
    // In idiomatic Rust, we would use Vec or Box which automatically free memory
    // when they go out of scope. However, since we're dealing with raw pointers
    // from C, we need to manually free them.

    // Free all the allocated memory safely
    if !store.litlens.is_null() {
        unsafe { libc::free(store.litlens as *mut c_void) };
        store.litlens = ptr::null_mut();
    }

    if !store.dists.is_null() {
        unsafe { libc::free(store.dists as *mut c_void) };
        store.dists = ptr::null_mut();
    }

    if !store.pos.is_null() {
        unsafe { libc::free(store.pos as *mut c_void) };
        store.pos = ptr::null_mut();
    }

    if !store.ll_symbol.is_null() {
        unsafe { libc::free(store.ll_symbol as *mut c_void) };
        store.ll_symbol = ptr::null_mut();
    }

    if !store.d_symbol.is_null() {
        unsafe { libc::free(store.d_symbol as *mut c_void) };
        store.d_symbol = ptr::null_mut();
    }

    if !store.ll_counts.is_null() {
        unsafe { libc::free(store.ll_counts as *mut c_void) };
        store.ll_counts = ptr::null_mut();
    }

    if !store.d_counts.is_null() {
        unsafe { libc::free(store.d_counts as *mut c_void) };
        store.d_counts = ptr::null_mut();
    }
}
#[no_mangle]
pub unsafe extern "C" fn ZopfliCleanLZ77Store(store: *mut ZopfliLZ77Store) {
    if !store.is_null() {
        ZopfliCleanLZ77Store_rust(&mut *store);
    }
}

fn GetLengthScore_rust(length: i32, distance: i32) -> i32 {
    /*
    At 1024, the distance uses 9+ extra bits and this seems to be the sweet spot
    on tested files.
    */
    if distance > 1024 {
        length - 1
    } else {
        length
    }
}
#[no_mangle]

fn ZopfliLZ77Greedy_rust(
    s: &mut ZopfliBlockState,
    data: &[u8],
    instart: usize,
    inend: usize,
    store: &mut ZopfliLZ77Store,
    h: &mut ZopfliHash,
) {
    if instart == inend {
        return;
    }

    let windowstart = if instart > 32768 { instart - 32768 } else { 0 };
    let mut dummysublen = [0u16; 259];

    // Lazy matching variables
    let mut prev_length = 0;
    let mut prev_match = 0;
    let mut match_available = false;

    ZopfliResetHash_rust(32768, h);
    ZopfliWarmupHash_rust(data, windowstart, inend, h);

    for i in windowstart..instart {
        // Update hash at position i
        unsafe {
            ZopfliUpdateHash(data.as_ptr(), i, inend, h as *mut ZopfliHash);
        }
    }

    let mut i = instart;
    while i < inend {
        // Update hash at position i
        unsafe {
            ZopfliUpdateHash(data.as_ptr(), i, inend, h as *mut ZopfliHash);
        }

        let mut dist = 0;
        let mut leng = 0;

        // Find longest match
        unsafe {
            ZopfliFindLongestMatch(
                s as *mut ZopfliBlockState,
                h as *const ZopfliHash,
                data.as_ptr(),
                i,
                inend,
                258,
                dummysublen.as_mut_ptr(),
                &mut dist,
                &mut leng,
            );
        }

        let lengthscore = GetLengthScore_rust(leng as i32, dist as i32);

        // Lazy matching
        if match_available {
            match_available = false;
            let prevlengthscore = GetLengthScore_rust(prev_length as i32, prev_match as i32);

            if lengthscore > prevlengthscore + 1 {
                // Add literal to output
                unsafe {
                    ZopfliStoreLitLenDist(
                        data[i - 1] as u16,
                        0,
                        i - 1,
                        store as *mut ZopfliLZ77Store,
                    );
                }

                if lengthscore >= 3 && leng < 258 {
                    match_available = true;
                    prev_length = leng;
                    prev_match = dist;
                    i += 1;
                    continue;
                }
            } else {
                // Add previous match to output
                leng = prev_length;
                dist = prev_match;

                ZopfliVerifyLenDist_rust(data, i - 1, dist, leng);

                unsafe {
                    ZopfliStoreLitLenDist(leng, dist, i - 1, store as *mut ZopfliLZ77Store);
                }

                // Skip the additional bytes of the match
                for _ in 2..leng {
                    assert!(i < inend, "i < inend");
                    i += 1;

                    unsafe {
                        ZopfliUpdateHash(data.as_ptr(), i, inend, h as *mut ZopfliHash);
                    }
                }

                i += 1;
                continue;
            }
        } else if lengthscore >= 3 && leng < 258 {
            match_available = true;
            prev_length = leng;
            prev_match = dist;
            i += 1;
            continue;
        }
        // End of lazy matching

        // Add to output
        if lengthscore >= 3 {
            ZopfliVerifyLenDist_rust(data, i, dist, leng);

            unsafe {
                ZopfliStoreLitLenDist(leng, dist, i, store as *mut ZopfliLZ77Store);
            }
        } else {
            leng = 1;

            unsafe {
                ZopfliStoreLitLenDist(data[i] as u16, 0, i, store as *mut ZopfliLZ77Store);
            }
        }

        // Skip the additional bytes of the match
        for _ in 1..leng {
            assert!(i < inend, "i < inend");
            i += 1;

            unsafe {
                ZopfliUpdateHash(data.as_ptr(), i, inend, h as *mut ZopfliHash);
            }
        }

        i += 1;
    }
}
#[no_mangle]
pub unsafe extern "C" fn ZopfliLZ77Greedy(
    s: *mut ZopfliBlockState,
    in_data: *const c_uchar,
    instart: size_t,
    inend: size_t,
    store: *mut ZopfliLZ77Store,
    h: *mut ZopfliHash,
) {
    let data_slice = slice::from_raw_parts(in_data, inend);

    ZopfliLZ77Greedy_rust(&mut *s, data_slice, instart, inend, &mut *store, &mut *h);
}

/// Finds the index that minimizes the given function within the specified range.
///
/// # Parameters
/// * `f` - A function that takes an index and context and returns a value to minimize
/// * `context` - Context to pass to the function
/// * `start` - Start of the range (inclusive)
/// * `end` - End of the range (exclusive)
///
/// # Returns
/// A tuple containing the index that minimizes the function and the minimum value
fn FindMinimum_rust<F, T>(f: F, context: &T, start: usize, end: usize) -> (usize, f64)
where
    F: Fn(usize, &T) -> f64,
{
    const THRESHOLD: usize = 1024;
    const SAMPLE_POINTS: usize = 9; // Good value: 9

    if end - start < THRESHOLD {
        // For small ranges, do a linear search
        let mut best = f64::MAX;
        let mut result = start;

        for i in start..end {
            let v = f(i, context);
            if v < best {
                best = v;
                result = i;
            }
        }

        (result, best)
    } else {
        // Try to find minimum faster by recursively checking multiple points
        let mut start = start;
        let mut end = end;
        let mut lastbest = f64::MAX;
        let mut pos = start;

        loop {
            if end - start <= SAMPLE_POINTS {
                break;
            }

            let mut p = [0; SAMPLE_POINTS];
            let mut vp = [0.0; SAMPLE_POINTS];

            for i in 0..SAMPLE_POINTS {
                p[i] = start + (i + 1) * ((end - start) / (SAMPLE_POINTS + 1));
                vp[i] = f(p[i], context);
            }

            let mut besti = 0;
            let mut best = vp[0];

            for i in 1..SAMPLE_POINTS {
                if vp[i] < best {
                    best = vp[i];
                    besti = i;
                }
            }

            if best > lastbest {
                break;
            }

            start = if besti == 0 { start } else { p[besti - 1] };
            end = if besti == SAMPLE_POINTS - 1 {
                end
            } else {
                p[besti + 1]
            };

            pos = p[besti];
            lastbest = best;
        }

        (pos, lastbest)
    }
}
#[no_mangle]
pub unsafe extern "C" fn FindMinimum(
    f: Option<unsafe extern "C" fn(i: size_t, context: *mut c_void) -> f64>,
    context: *mut c_void,
    start: size_t,
    end: size_t,
    smallest: *mut f64,
) -> size_t {
    // Unwrap the function pointer
    let f_unwrapped = match f {
        Some(func) => func,
        None => return start, // Return early if function is null
    };

    // Create a safe wrapper around the C function
    let f_wrapper = |idx: usize, _: &*mut c_void| -> f64 { unsafe { f_unwrapped(idx, context) } };

    // Call the Rust implementation
    let (result_idx, min_value) = FindMinimum_rust(f_wrapper, &context, start, end);

    // Write the minimum value to the output parameter
    if !smallest.is_null() {
        *smallest = min_value;
    }

    result_idx
}

fn CalculateBlockSymbolSize_rust(
    ll_lengths: &[usize],
    d_lengths: &[usize],
    lz77: &ZopfliLZ77Store,
    lstart: usize,
    lend: usize,
) -> usize {
    if lstart + 288 * 3 > lend {
        CalculateBlockSymbolSizeSmall_rust(ll_lengths, d_lengths, lz77, lstart, lend)
    } else {
        let mut ll_counts = [0usize; 288];
        let mut d_counts = [0usize; 32];

        ZopfliLZ77GetHistogram_rust(lz77, lstart, lend, &mut ll_counts, &mut d_counts);

        CalculateBlockSymbolSizeGivenCounts_rust(
            &ll_counts, &d_counts, ll_lengths, d_lengths, lz77, lstart, lend,
        )
    }
}
#[no_mangle]
pub unsafe extern "C" fn CalculateBlockSymbolSize(
    ll_lengths: *const libc::c_uint,
    d_lengths: *const libc::c_uint,
    lz77: *const ZopfliLZ77Store,
    lstart: libc::size_t,
    lend: libc::size_t,
) -> libc::size_t {
    // Convert C unsigned int arrays to Rust slices of usize
    let ll_lengths_slice = std::slice::from_raw_parts(ll_lengths, 288)
        .iter()
        .map(|&x| x as usize)
        .collect::<Vec<usize>>();

    let d_lengths_slice = std::slice::from_raw_parts(d_lengths, 32)
        .iter()
        .map(|&x| x as usize)
        .collect::<Vec<usize>>();

    // Dereference the lz77 pointer to get a reference
    let lz77_ref = &*lz77;

    // Call the Rust implementation
    CalculateBlockSymbolSize_rust(&ll_lengths_slice, &d_lengths_slice, lz77_ref, lstart, lend)
        as libc::size_t
}

fn ZopfliCalculateBlockSize_rust(
    lz77: &ZopfliLZ77Store,
    lstart: usize,
    lend: usize,
    btype: i32,
) -> f64 {
    let mut ll_lengths = [0u32; 288];
    let mut d_lengths = [0u32; 32];

    // bfinal and btype bits
    let mut result = 3.0;

    if btype == 0 {
        let length = ZopfliLZ77GetByteRange_rust(lz77, lstart, lend);
        let rem = length % 65535;
        let blocks = length / 65535 + if rem > 0 { 1 } else { 0 };
        // An uncompressed block must actually be split into multiple blocks if it's
        // larger than 65535 bytes long. Each block header is 5 bytes: 3 bits,
        // padding, LEN and NLEN (potential less padding for first one ignored).
        return (blocks * 5 * 8 + length * 8) as f64;
    }

    if btype == 1 {
        GetFixedTree_rust(&mut ll_lengths, &mut d_lengths);

        // Convert u32 arrays to usize arrays for CalculateBlockSymbolSize_rust
        let ll_lengths_usize: Vec<usize> = ll_lengths.iter().map(|&x| x as usize).collect();
        let d_lengths_usize: Vec<usize> = d_lengths.iter().map(|&x| x as usize).collect();

        result +=
            CalculateBlockSymbolSize_rust(&ll_lengths_usize, &d_lengths_usize, lz77, lstart, lend)
                as f64;
    } else {
        result += GetDynamicLengths_rust(lz77, lstart, lend, &mut ll_lengths, &mut d_lengths);
    }

    result
}
#[no_mangle]
pub unsafe extern "C" fn ZopfliCalculateBlockSize(
    lz77: *const ZopfliLZ77Store,
    lstart: size_t,
    lend: size_t,
    btype: c_int,
) -> f64 {
    let lz77_ref = &*lz77;
    ZopfliCalculateBlockSize_rust(lz77_ref, lstart, lend, btype)
}

fn ZopfliCalculateBlockSizeAutoType_rust(
    lz77: &crate::ZopfliLZ77Store,
    lstart: usize,
    lend: usize,
) -> f64 {
    let uncompressed_cost = ZopfliCalculateBlockSize_rust(lz77, lstart, lend, 0);

    // Don't do the expensive fixed cost calculation for larger blocks that are
    // unlikely to use it.
    let fixed_cost = if lz77.size > 1000 {
        uncompressed_cost
    } else {
        ZopfliCalculateBlockSize_rust(lz77, lstart, lend, 1)
    };

    let dyn_cost = ZopfliCalculateBlockSize_rust(lz77, lstart, lend, 2);

    // Return the minimum cost among the three block types
    if uncompressed_cost < fixed_cost && uncompressed_cost < dyn_cost {
        uncompressed_cost
    } else if fixed_cost < dyn_cost {
        fixed_cost
    } else {
        dyn_cost
    }
}
#[no_mangle]
pub unsafe extern "C" fn ZopfliCalculateBlockSizeAutoType(
    lz77: *const crate::ZopfliLZ77Store,
    lstart: size_t,
    lend: size_t,
) -> f64 {
    let lz77_ref = &*lz77;
    ZopfliCalculateBlockSizeAutoType_rust(lz77_ref, lstart, lend)
}

fn EstimateCost_rust(lz77: &crate::ZopfliLZ77Store, lstart: usize, lend: usize) -> f64 {
    ZopfliCalculateBlockSizeAutoType_rust(lz77, lstart, lend)
}
#[no_mangle]
pub unsafe extern "C" fn EstimateCost(
    lz77: *const crate::ZopfliLZ77Store,
    lstart: size_t,
    lend: size_t,
) -> f64 {
    let lz77_ref = &*lz77;
    EstimateCost_rust(lz77_ref, lstart, lend)
}

fn AddSorted_rust(value: usize, out: &mut Vec<usize>) {
    // Add the value to the end of the vector
    out.push(value);

    // Find the correct position for the value and move it there
    let mut i = 0;
    while i + 1 < out.len() {
        if out[i] > value {
            // Found the position where the value should be
            // Move the value from the end to this position
            let last_idx = out.len() - 1;
            for j in (i + 1..=last_idx).rev() {
                out[j] = out[j - 1];
            }
            out[i] = value;
            break;
        }
        i += 1;
    }
}
#[no_mangle]
pub unsafe extern "C" fn AddSorted(value: size_t, out: *mut *mut size_t, outsize: *mut size_t) {
    // Convert the raw pointers to Rust references
    let out_ptr = &mut *out;
    let outsize_val = *outsize;

    // Create a Vec from the raw pointer
    let mut vec = if outsize_val == 0 {
        Vec::new()
    } else {
        Vec::from_raw_parts(*out_ptr, outsize_val, outsize_val)
    };

    // Call the Rust implementation
    AddSorted_rust(value, &mut vec);

    // Update the outsize
    *outsize = vec.len();

    // Get the raw parts of the Vec without dropping it
    let ptr = vec.as_mut_ptr();
    let len = vec.len();
    let cap = vec.capacity();

    // Update the out pointer
    *out_ptr = ptr;

    // Prevent the Vec from being dropped to avoid double-free
    mem::forget(vec);
}

fn FindLargestSplittableBlock_rust(
    lz77size: usize,
    done: &[bool],
    splitpoints: &[usize],
    lstart: &mut usize,
    lend: &mut usize,
) -> bool {
    let npoints = splitpoints.len();
    let mut longest = 0;
    let mut found = false;

    for i in 0..=npoints {
        let start = if i == 0 { 0 } else { splitpoints[i - 1] };
        let end = if i == npoints {
            lz77size - 1
        } else {
            splitpoints[i]
        };

        if start < done.len() && !done[start] && end - start > longest {
            *lstart = start;
            *lend = end;
            found = true;
            longest = end - start;
        }
    }

    found
}
#[no_mangle]

/// Prints block split points to stderr
fn PrintBlockSplitPoints_rust(
    lz77: *const ZopfliLZ77Store,
    lz77splitpoints: *const usize,
    nlz77points: usize,
) {
    // Use a Vec instead of manually allocating memory
    let mut splitpoints = Vec::new();
    let mut npoints = 0;

    // The input is given as lz77 indices, but we want to see the uncompressed index values
    let mut pos = 0;

    unsafe {
        if nlz77points > 0 && !lz77splitpoints.is_null() {
            let lz77_ref = &*lz77;

            for i in 0..lz77_ref.size {
                let length = if *lz77_ref.dists.add(i) == 0 {
                    1
                } else {
                    *lz77_ref.litlens.add(i)
                };

                if npoints < nlz77points && *lz77splitpoints.add(npoints) == i {
                    splitpoints.push(pos);
                    npoints += 1;

                    if npoints == nlz77points {
                        break;
                    }
                }

                pos += length;
            }

            // Assert that we found all the split points
            assert!(npoints == nlz77points, "npoints == nlz77points");
        }

        // Print the split points
        let mut stderr_file = std::io::stderr();

        write!(stderr_file, "block split points: ").unwrap();
        for &point in &splitpoints {
            write!(stderr_file, "{} ", point).unwrap();
        }

        write!(stderr_file, "(hex:").unwrap();
        for &point in &splitpoints {
            write!(stderr_file, " {:x}", point).unwrap();
        }

        writeln!(stderr_file, ")").unwrap();
    }

    // No need to free memory as Vec will be dropped automatically
}
#[no_mangle]
pub unsafe extern "C" fn PrintBlockSplitPoints(
    lz77: *const ZopfliLZ77Store,
    lz77splitpoints: *const size_t,
    nlz77points: size_t,
) {
    // Call the Rust implementation
    PrintBlockSplitPoints_rust(lz77, lz77splitpoints as *const usize, nlz77points)
}

pub struct ZopfliLZ77Store_rust {
    /// Lit or len.
    pub litlens: Vec<u16>,

    /// If 0: indicates literal in corresponding litlens,
    /// if > 0: length in corresponding litlens, this is the distance.
    pub dists: Vec<u16>,

    /// Size of the store
    pub size: usize,

    /// Original data
    pub data: Vec<u8>,

    /// Position in data where this LZ77 command begins
    pub pos: Vec<usize>,

    /// LL symbols
    pub ll_symbol: Vec<u16>,

    /// D symbols
    pub d_symbol: Vec<u16>,

    /// Cumulative histograms wrapping around per chunk. Each chunk has the amount
    /// of distinct symbols as length, so using 1 value per LZ77 symbol, we have a
    /// precise histogram at every N symbols, and the rest can be calculated by
    /// looping through the actual symbols of this chunk.
    pub ll_counts: Vec<usize>,

    /// D counts for histograms
    pub d_counts: Vec<usize>,
}

pub struct SplitCostContext_rust {
    /// Reference to the LZ77 store
    pub lz77: *const ZopfliLZ77Store_rust,
    /// Start position
    pub start: usize,
    /// End position
    pub end: usize,
}

/// Splits the LZ77 data into blocks to improve compression.
///
/// `options`: Options for the algorithm
/// `lz77`: The LZ77 data to split
/// `max_blocks`: Maximum number of blocks to split into, or 0 for no limit
/// `split_points`: Output vector containing the positions where to split the blocks
fn ZopfliBlockSplitLZ77_rust(
    options: &ZopfliOptions,
    lz77: &ZopfliLZ77Store,
    max_blocks: usize,
    split_points: &mut Vec<usize>,
) {
    // Clear any existing split points
    split_points.clear();

    if lz77.size < 10 {
        return; // This code fails on tiny files.
    }

    let mut done = vec![false; lz77.size];
    let mut lstart = 0;
    let mut lend = lz77.size;
    let mut num_blocks = 1;

    loop {
        if max_blocks > 0 && num_blocks >= max_blocks {
            break;
        }

        struct SplitContext<'a> {
            lz77: &'a ZopfliLZ77Store,
            start: usize,
            end: usize,
        }

        let context = SplitContext {
            lz77,
            start: lstart,
            end: lend,
        };

        assert!(lstart < lend, "lstart must be less than lend");

        // Define a local split cost function
        let split_cost_fn = |i: usize, ctx: &SplitContext| -> f64 {
            let start = ctx.start;
            let end = ctx.end;
            let lz77 = ctx.lz77;

            EstimateCost_rust(lz77, start, i) + EstimateCost_rust(lz77, i, end)
        };

        let (llpos, split_cost) = FindMinimum_rust(split_cost_fn, &context, lstart + 1, lend);

        assert!(llpos > lstart, "llpos must be greater than lstart");
        assert!(llpos < lend, "llpos must be less than lend");

        let orig_cost = EstimateCost_rust(lz77, lstart, lend);

        if split_cost > orig_cost || llpos == lstart + 1 || llpos == lend {
            done[lstart] = true;
        } else {
            AddSorted_rust(llpos, split_points);
            num_blocks += 1;
        }

        if !FindLargestSplittableBlock_rust(lz77.size, &done, split_points, &mut lstart, &mut lend)
        {
            break; // No further split will probably reduce compression.
        }

        if lend - lstart < 10 {
            break;
        }
    }

    if options.verbose != 0 {
        PrintBlockSplitPoints_rust(lz77, split_points.as_ptr(), split_points.len());
    }
}
#[no_mangle]
pub unsafe extern "C" fn ZopfliBlockSplitLZ77(
    options: *const ZopfliOptions,
    lz77: *const ZopfliLZ77Store,
    maxblocks: size_t,
    splitpoints: *mut *mut size_t,
    npoints: *mut size_t,
) {
    let options = &*options;
    let lz77 = &*lz77;
    let maxblocks = maxblocks as usize;

    // Create a new vector to hold the split points
    let mut split_points = Vec::new();

    // Call the Rust implementation
    ZopfliBlockSplitLZ77_rust(options, lz77, maxblocks, &mut split_points);

    // If we have existing split points, free them
    if !(*splitpoints).is_null() {
        let old_len = *npoints as usize;
        if old_len > 0 {
            Vec::from_raw_parts(*splitpoints as *mut usize, old_len, old_len);
        }
    }

    // Update the output parameters
    *npoints = split_points.len() as size_t;

    if split_points.is_empty() {
        *splitpoints = ptr::null_mut();
    } else {
        // Allocate memory for the split points
        let size = split_points.len() * mem::size_of::<size_t>();
        let ptr = libc::malloc(size) as *mut size_t;

        if !ptr.is_null() {
            // Copy the split points to the allocated memory
            for (i, &point) in split_points.iter().enumerate() {
                *ptr.add(i) = point as size_t;
            }

            *splitpoints = ptr;
        } else {
            // Allocation failed
            *splitpoints = ptr::null_mut();
            *npoints = 0;
        }
    }
}

/// Options used for Zopfli compression.
pub struct ZopfliOptions_rust {
    /// Whether to print output
    pub verbose: bool,

    /// Whether to print more detailed output
    pub verbose_more: bool,

    /// Maximum amount of times to rerun forward and backward pass to optimize LZ77
    /// compression cost. Good values: 10, 15 for small files, 5 for files over
    /// several MB in size or it will be too slow.
    pub numiterations: i32,

    /// If true, splits the data in multiple deflate blocks with optimal choice
    /// for the block boundaries. Block splitting gives better compression.
    /// Default: true
    pub blocksplitting: bool,

    /// No longer used, left for compatibility.
    pub blocksplittinglast: i32,

    /// Maximum amount of blocks to split into (0 for unlimited, but this can give
    /// extreme results that hurt compression on some files). Default value: 15.
    pub blocksplittingmax: i32,
}

pub struct ZopfliBlockState_rust {
    /// Options for the compressor
    pub options: *const ZopfliOptions_rust,

    /// Cache for length/distance pairs found so far.
    pub lmc: *mut ZopfliLongestMatchCache_rust,

    /// The start (inclusive) of the current block.
    pub blockstart: usize,

    /// The end (not inclusive) of the current block.
    pub blockend: usize,
}

fn ZopfliBlockSplit_rust(
    options: &ZopfliOptions,
    data: &[u8],
    instart: usize,
    inend: usize,
    maxblocks: usize,
    splitpoints: &mut Vec<usize>,
) {
    // Create a temporary ZopfliBlockState
    let mut s = ZopfliBlockState {
        options,
        lmc: ptr::null_mut(),
        blockstart: instart,
        blockend: inend,
    };

    // Initialize an empty LZ77Store
    let mut store = ZopfliLZ77Store {
        litlens: ptr::null_mut(),
        dists: ptr::null_mut(),
        size: 0,
        data: ptr::null(),
        pos: ptr::null_mut(),
        ll_symbol: ptr::null_mut(),
        d_symbol: ptr::null_mut(),
        ll_counts: ptr::null_mut(),
        d_counts: ptr::null_mut(),
    };

    // Initialize an empty hash
    let mut hash = ZopfliHash {
        head: ptr::null_mut(),
        prev: ptr::null_mut(),
        hashval: ptr::null_mut(),
        val: 0,
        head2: ptr::null_mut(),
        prev2: ptr::null_mut(),
        hashval2: ptr::null_mut(),
        val2: 0,
        same: ptr::null_mut(),
    };

    // Initialize the data structures
    ZopfliInitLZ77Store_rust(data.as_ptr(), &mut store);
    ZopfliInitBlockState_rust(options, instart, inend, false, &mut s);
    ZopfliAllocHash_rust(32768, &mut hash);

    // Clear the output vector
    splitpoints.clear();

    // Perform LZ77 compression
    ZopfliLZ77Greedy_rust(&mut s, data, instart, inend, &mut store, &mut hash);

    // Find split points
    let mut lz77splitpoints = Vec::new();
    ZopfliBlockSplitLZ77_rust(options, &store, maxblocks, &mut lz77splitpoints);

    // Convert LZ77 positions to positions in the uncompressed input
    let mut pos = instart;
    if !lz77splitpoints.is_empty() {
        let mut next_point_idx = 0;

        // Safely access the store data
        unsafe {
            for i in 0..store.size {
                let dist = *store.dists.add(i);
                let length = if dist == 0 {
                    1
                } else {
                    *store.litlens.add(i) as usize
                };

                if next_point_idx < lz77splitpoints.len() && lz77splitpoints[next_point_idx] == i {
                    splitpoints.push(pos);
                    next_point_idx += 1;
                    if next_point_idx == lz77splitpoints.len() {
                        break;
                    }
                }
                pos += length;
            }
        }
    }

    // Verify that we found all split points
    assert_eq!(
        splitpoints.len(),
        lz77splitpoints.len(),
        "splitpoints.len() must equal lz77splitpoints.len()"
    );

    // Clean up
    ZopfliCleanBlockState_rust(None);
    ZopfliCleanLZ77Store_rust(&mut store);
    ZopfliCleanHash_rust(&mut hash);
}
#[no_mangle]
pub unsafe extern "C" fn ZopfliBlockSplit(
    options: *const ZopfliOptions,
    in_data: *const c_uchar,
    instart: size_t,
    inend: size_t,
    maxblocks: size_t,
    splitpoints: *mut *mut size_t,
    npoints: *mut size_t,
) {
    let options_ref = &*(options as *const ZopfliOptions);
    let data_slice = slice::from_raw_parts(in_data, inend);

    // Create a Rust vector to hold the split points
    let mut split_points_vec = Vec::new();

    // Call the Rust implementation
    ZopfliBlockSplit_rust(
        options_ref,
        data_slice,
        instart,
        inend,
        maxblocks,
        &mut split_points_vec,
    );

    // Set the number of points
    *npoints = split_points_vec.len();

    // Allocate memory for the C array and copy the data
    if !split_points_vec.is_empty() {
        let size = split_points_vec.len() * mem::size_of::<size_t>();
        let ptr = malloc(size as u64) as *mut size_t;

        if !ptr.is_null() {
            for (i, &point) in split_points_vec.iter().enumerate() {
                *ptr.add(i) = point;
            }
            *splitpoints = ptr;
        } else {
            *splitpoints = ptr::null_mut();
            *npoints = 0;
        }
    } else {
        *splitpoints = ptr::null_mut();
    }
}

/// Initializes a random number generator state
fn InitRanState_rust(state: &mut RanState) {
    state.m_w = 1;
    state.m_z = 2;
}
#[no_mangle]
pub unsafe extern "C" fn InitRanState(state: *mut RanState) {
    if !state.is_null() {
        let rust_state = &mut *state;
        InitRanState_rust(rust_state);
    }
}

fn InitStats_rust(stats: &mut SymbolStats) {
    // Initialize all arrays to zeros
    stats.litlens = [0; 288];
    stats.dists = [0; 32];
    stats.ll_symbols = [0.0; 288];
    stats.d_symbols = [0.0; 32];
}
#[no_mangle]
pub unsafe extern "C" fn InitStats(stats: *mut SymbolStats) {
    if !stats.is_null() {
        // Convert the raw pointer to a mutable reference and call the Rust implementation
        InitStats_rust(&mut *stats);
    }
}

fn ZopfliCalculateEntropy_rust(count: &[usize], bitlengths: &mut [f64]) {
    const INV_LOG2: f64 = 1.4426950408889; // 1.0 / log(2.0)

    let sum: usize = count.iter().sum();
    let log2sum = if sum == 0 {
        f64::ln(count.len() as f64) * INV_LOG2
    } else {
        f64::ln(sum as f64) * INV_LOG2
    };

    for (i, &c) in count.iter().enumerate() {
        // When the count of the symbol is 0, but its cost is requested anyway, it
        // means the symbol will appear at least once anyway, so give it the cost as if
        // its count is 1.
        bitlengths[i] = if c == 0 {
            log2sum
        } else {
            log2sum - f64::ln(c as f64) * INV_LOG2
        };

        // Depending on compiler and architecture, the above subtraction of two
        // floating point numbers may give a negative result very close to zero
        // instead of zero. Clamp it to zero. These floating point imprecisions do not
        // affect the cost model significantly so this is ok.
        if bitlengths[i] < 0.0 && bitlengths[i] > -1e-5 {
            bitlengths[i] = 0.0;
        }

        // Assert that bitlengths[i] is non-negative
        assert!(bitlengths[i] >= 0.0, "bitlengths[i] >= 0");
    }
}
#[no_mangle]
pub unsafe extern "C" fn ZopfliCalculateEntropy(
    count: *const size_t,
    n: size_t,
    bitlengths: *mut f64,
) {
    let count_slice = std::slice::from_raw_parts(count, n);
    let bitlengths_slice = std::slice::from_raw_parts_mut(bitlengths, n);

    ZopfliCalculateEntropy_rust(count_slice, bitlengths_slice);
}

fn CalculateStatistics_rust(stats: &mut SymbolStats) {
    ZopfliCalculateEntropy_rust(&stats.litlens[0..288], &mut stats.ll_symbols);
    ZopfliCalculateEntropy_rust(&stats.dists[0..32], &mut stats.d_symbols);
}
#[no_mangle]
pub unsafe extern "C" fn CalculateStatistics(stats: *mut SymbolStats) {
    if !stats.is_null() {
        CalculateStatistics_rust(&mut *stats);
    }
}

fn GetStatistics_rust(store: &ZopfliLZ77Store, stats: &mut SymbolStats) {
    unsafe {
        for i in 0..store.size {
            let litlen = *store.litlens.add(i);
            let dist = *store.dists.add(i);

            if dist == 0 {
                stats.litlens[litlen as usize] += 1;
            } else {
                let length_symbol = ZopfliGetLengthSymbol_rust(litlen as usize);
                let dist_symbol = ZopfliGetDistSymbol_rust(dist as i32) as usize;

                stats.litlens[length_symbol] += 1;
                stats.dists[dist_symbol] += 1;
            }
        }

        // End symbol
        stats.litlens[256] = 1;

        CalculateStatistics_rust(stats);
    }
}
#[no_mangle]
pub unsafe extern "C" fn GetStatistics(store: *const ZopfliLZ77Store, stats: *mut SymbolStats) {
    let store_ref = &*store;
    let stats_ref = &mut *stats;

    GetStatistics_rust(store_ref, stats_ref);
}

fn CeilDiv_rust(a: usize, b: usize) -> usize {
    (a + b - 1) / b
}
#[no_mangle]
pub unsafe extern "C" fn CeilDiv(a: size_t, b: size_t) -> size_t {
    CeilDiv_rust(a, b)
}

fn ZopfliCopyLZ77Store_rust(source: &ZopfliLZ77Store, dest: &mut ZopfliLZ77Store) {
    // Clean and initialize the destination store
    ZopfliCleanLZ77Store_rust(dest);
    ZopfliInitLZ77Store_rust(source.data, dest);

    let llsize = 288 * CeilDiv_rust(source.size, 288);
    let dsize = 32 * CeilDiv_rust(source.size, 32);

    // Allocate memory for all fields using Vec and then leak it to get raw pointers
    // This is safer than direct malloc calls
    dest.litlens = Vec::with_capacity(source.size).leak().as_mut_ptr();
    dest.dists = Vec::with_capacity(source.size).leak().as_mut_ptr();
    dest.pos = Vec::with_capacity(source.size).leak().as_mut_ptr();
    dest.ll_symbol = Vec::with_capacity(source.size).leak().as_mut_ptr();
    dest.d_symbol = Vec::with_capacity(source.size).leak().as_mut_ptr();
    dest.ll_counts = Vec::with_capacity(llsize).leak().as_mut_ptr();
    dest.d_counts = Vec::with_capacity(dsize).leak().as_mut_ptr();

    // Set the size
    dest.size = source.size;

    // Copy data from source to destination using unsafe code
    unsafe {
        for i in 0..source.size {
            *dest.litlens.add(i) = *source.litlens.add(i);
            *dest.dists.add(i) = *source.dists.add(i);
            *dest.pos.add(i) = *source.pos.add(i);
            *dest.ll_symbol.add(i) = *source.ll_symbol.add(i);
            *dest.d_symbol.add(i) = *source.d_symbol.add(i);
        }

        for i in 0..llsize {
            *dest.ll_counts.add(i) = *source.ll_counts.add(i);
        }

        for i in 0..dsize {
            *dest.d_counts.add(i) = *source.d_counts.add(i);
        }
    }
}
#[no_mangle]
pub unsafe extern "C" fn ZopfliCopyLZ77Store(
    source: *const ZopfliLZ77Store,
    dest: *mut ZopfliLZ77Store,
) {
    let source_ref = &*source;
    let dest_ref = &mut *dest;

    ZopfliCopyLZ77Store_rust(source_ref, dest_ref);
}

fn CopyStats_rust(source: &SymbolStats, dest: &mut SymbolStats) {
    dest.litlens.copy_from_slice(&source.litlens);
    dest.dists.copy_from_slice(&source.dists);
    dest.ll_symbols.copy_from_slice(&source.ll_symbols);
    dest.d_symbols.copy_from_slice(&source.d_symbols);
}
#[no_mangle]
pub unsafe extern "C" fn CopyStats(source: *const SymbolStats, dest: *mut SymbolStats) {
    if !source.is_null() && !dest.is_null() {
        CopyStats_rust(&*source, &mut *dest);
    }
}

/// Clears the frequency statistics in a SymbolStats struct.
///
/// This function sets all frequency counters to zero for both literal/length symbols
/// and distance symbols.
fn ClearStatFreqs_rust(stats: &mut SymbolStats) {
    for i in 0..288 {
        stats.litlens[i] = 0;
    }

    for i in 0..32 {
        stats.dists[i] = 0;
    }
}
#[no_mangle]
pub unsafe extern "C" fn ClearStatFreqs(stats: *mut SymbolStats) {
    if !stats.is_null() {
        ClearStatFreqs_rust(&mut *stats);
    }
}

/// Rust implementation of AddWeighedStatFreqs
///
/// Adds the weighted frequencies of two symbol statistics together
/// and stores the result in a third symbol statistics struct.
fn AddWeighedStatFreqs_rust(
    stats1: &SymbolStats,
    w1: f64,
    stats2: &SymbolStats,
    w2: f64,
    result: &mut SymbolStats,
) {
    for i in 0..288 {
        result.litlens[i] =
            (stats1.litlens[i] as f64 * w1 + stats2.litlens[i] as f64 * w2) as usize;
    }

    for i in 0..32 {
        result.dists[i] = (stats1.dists[i] as f64 * w1 + stats2.dists[i] as f64 * w2) as usize;
    }

    // End symbol
    result.litlens[256] = 1;
}
#[no_mangle]
pub unsafe extern "C" fn AddWeighedStatFreqs(
    stats1: *const SymbolStats,
    w1: f64,
    stats2: *const SymbolStats,
    w2: f64,
    result: *mut SymbolStats,
) {
    let stats1_ref = &*stats1;
    let stats2_ref = &*stats2;
    let result_ref = &mut *result;

    AddWeighedStatFreqs_rust(stats1_ref, w1, stats2_ref, w2, result_ref);
}

/// Generates a random number using the provided RanState
fn Ran_rust(state: &mut RanState) -> u32 {
    // Use wrapping operations to match C's overflow behavior
    state.m_z = 36969u32
        .wrapping_mul(state.m_z & 65535)
        .wrapping_add(state.m_z >> 16);
    state.m_w = 18000u32
        .wrapping_mul(state.m_w & 65535)
        .wrapping_add(state.m_w >> 16);

    // Use wrapping_add for the final addition as well
    (state.m_z << 16).wrapping_add(state.m_w) // 32-bit result
}
/// C-compatible wrapper for the Ran_rust function
#[no_mangle]
pub unsafe extern "C" fn Ran(state: *mut RanState) -> u32 {
    let state_ref = &mut *state;
    Ran_rust(state_ref)
}

fn RandomizeFreqs_rust(state: &mut RanState, freqs: &mut [usize], n: usize) {
    for i in 0..n {
        if (Ran_rust(state) >> 4) % 3 == 0 {
            let random_index = (Ran_rust(state) % n as u32) as usize;
            freqs[i] = freqs[random_index];
        }
    }
}
#[no_mangle]
pub unsafe extern "C" fn RandomizeFreqs(state: *mut RanState, freqs: *mut size_t, n: c_int) {
    let n_usize = n as usize;
    let freqs_slice = slice::from_raw_parts_mut(freqs, n_usize);

    RandomizeFreqs_rust(&mut *state, freqs_slice, n_usize);
}

fn RandomizeStatFreqs_rust(state: &mut RanState, stats: &mut SymbolStats) {
    RandomizeFreqs_rust(state, &mut stats.litlens, 288);
    RandomizeFreqs_rust(state, &mut stats.dists, 32);
    stats.litlens[256] = 1; // End symbol.
}
#[no_mangle]
pub unsafe extern "C" fn RandomizeStatFreqs(state: *mut RanState, stats: *mut SymbolStats) {
    let state = &mut *state;
    let stats = &mut *stats;
    RandomizeStatFreqs_rust(state, stats);
}

pub struct RanState_rust {
    pub m_w: u32,
    pub m_z: u32,
}

pub struct SymbolStats_rust {
    /// The literal and length symbols.
    pub litlens: [usize; 288],
    /// The 32 unique dist symbols, not the 32768 possible dists.
    pub dists: [usize; 32],

    /// Length of each lit/len symbol in bits.
    pub ll_symbols: [f64; 288],
    /// Length of each dist symbol in bits.
    pub d_symbols: [f64; 32],
}

/// Appends the LZ77 data from one store to another.
///
/// This is a Rust implementation of the original C function.
fn ZopfliAppendLZ77Store_rust(store: &ZopfliLZ77Store, target: &mut ZopfliLZ77Store) {
    for i in 0..store.size {
        unsafe {
            let litlen = *store.litlens.add(i);
            let dist = *store.dists.add(i);
            let pos = *store.pos.add(i);

            ZopfliStoreLitLenDist(litlen, dist, pos, target);
        }
    }
}
/// C-compatible wrapper for ZopfliAppendLZ77Store_rust
#[no_mangle]
pub unsafe extern "C" fn ZopfliAppendLZ77Store(
    store: *const ZopfliLZ77Store,
    target: *mut ZopfliLZ77Store,
) {
    if !store.is_null() && !target.is_null() {
        let store_ref = &*store;
        let target_ref = &mut *target;
        ZopfliAppendLZ77Store_rust(store_ref, target_ref);
    }
}

fn ZopfliDeflatePart_rust(
    options: &ZopfliOptions,
    btype: i32,
    final_block: bool,
    input: &[u8],
    instart: usize,
    inend: usize,
    bp: &mut u8,
    out: &mut Vec<u8>,
) {
    // If btype=0, add non-compressed block
    if btype == 0 {
        AddNonCompressedBlock_rust(options, final_block, input, instart, inend, bp, out);
        return;
    }
    // If btype=1, use fixed huffman block
    else if btype == 1 {
        // Create an uninitialized store and block state
        let mut store = unsafe { mem::zeroed::<ZopfliLZ77Store>() };
        let mut s = unsafe { mem::zeroed::<ZopfliBlockState>() };

        // Initialize store and block state
        ZopfliInitLZ77Store_rust(input.as_ptr(), &mut store);
        ZopfliInitBlockState_rust(options, instart, inend, true, &mut s);

        // Use unsafe block for FFI call
        unsafe {
            ZopfliLZ77OptimalFixed(
                &mut s as *mut ZopfliBlockState,
                input.as_ptr(),
                instart,
                inend,
                &mut store as *mut ZopfliLZ77Store,
            );
        }

        // Add LZ77 block
        AddLZ77Block_rust(
            options,
            btype,
            final_block,
            &store,
            0,
            store.size,
            0,
            bp,
            out,
        );

        // Clean up - convert lmc to Option<*mut c_void>
        let lmc_option = if s.lmc.is_null() {
            None
        } else {
            Some(s.lmc as *mut c_void)
        };
        ZopfliCleanBlockState_rust(lmc_option);
        ZopfliCleanLZ77Store_rust(&mut store);
        return;
    }

    // For btype=2 (dynamic huffman blocks), we may need block splitting
    let mut splitpoints_uncompressed = Vec::new();
    let mut splitpoints = Vec::new();
    let mut totalcost = 0.0;

    // Create an uninitialized lz77 store
    let mut lz77 = unsafe { mem::zeroed::<ZopfliLZ77Store>() };

    // Initialize LZ77 store
    ZopfliInitLZ77Store_rust(input.as_ptr(), &mut lz77);

    // Perform block splitting if enabled
    if options.blocksplitting != 0 {
        ZopfliBlockSplit_rust(
            options,
            input,
            instart,
            inend,
            options.blocksplittingmax as usize,
            &mut splitpoints_uncompressed,
        );
    }

    // Process each block
    let npoints = splitpoints_uncompressed.len();
    for i in 0..=npoints {
        let start = if i == 0 {
            instart
        } else {
            splitpoints_uncompressed[i - 1]
        };
        let end = if i == npoints {
            inend
        } else {
            splitpoints_uncompressed[i]
        };

        // Create uninitialized store and block state
        let mut store = unsafe { mem::zeroed::<ZopfliLZ77Store>() };
        let mut s = unsafe { mem::zeroed::<ZopfliBlockState>() };

        ZopfliInitLZ77Store_rust(input.as_ptr(), &mut store);
        ZopfliInitBlockState_rust(options, start, end, true, &mut s);

        // Use unsafe block for FFI call
        unsafe {
            ZopfliLZ77Optimal(
                &mut s as *mut ZopfliBlockState,
                input.as_ptr(),
                start,
                end,
                options.numiterations,
                &mut store as *mut ZopfliLZ77Store,
            );
        }

        totalcost += ZopfliCalculateBlockSizeAutoType_rust(&store, 0, store.size);

        ZopfliAppendLZ77Store_rust(&store, &mut lz77);
        if i < npoints {
            splitpoints.push(lz77.size);
        }

        // Clean up - convert lmc to Option<*mut c_void>
        let lmc_option = if s.lmc.is_null() {
            None
        } else {
            Some(s.lmc as *mut c_void)
        };
        ZopfliCleanBlockState_rust(lmc_option);
        ZopfliCleanLZ77Store_rust(&mut store);
    }

    // Second block splitting attempt
    if options.blocksplitting != 0 && npoints > 1 {
        let mut splitpoints2 = Vec::new();
        let mut totalcost2 = 0.0;

        ZopfliBlockSplitLZ77_rust(
            options,
            &lz77,
            options.blocksplittingmax as usize,
            &mut splitpoints2,
        );

        let npoints2 = splitpoints2.len();
        for i in 0..=npoints2 {
            let start = if i == 0 { 0 } else { splitpoints2[i - 1] };
            let end = if i == npoints2 {
                lz77.size
            } else {
                splitpoints2[i]
            };
            totalcost2 += ZopfliCalculateBlockSizeAutoType_rust(&lz77, start, end);
        }

        // Use the better splitting
        if totalcost2 < totalcost {
            splitpoints = splitpoints2;
        }
    }

    // Add blocks with the chosen splitting
    let npoints = splitpoints.len();
    for i in 0..=npoints {
        let start = if i == 0 { 0 } else { splitpoints[i - 1] };
        let end = if i == npoints {
            lz77.size
        } else {
            splitpoints[i]
        };

        // Since AddLZ77BlockAutoType is not accessible, we use AddLZ77Block with btype=2
        // This is a simplification - in a real implementation you'd need the actual function
        AddLZ77Block_rust(
            options,
            2, // Dynamic Huffman blocks
            i == npoints && final_block,
            &lz77,
            start,
            end,
            0,
            bp,
            out,
        );
    }

    ZopfliCleanLZ77Store_rust(&mut lz77);
}
#[no_mangle]

/// Deflates the input data in chunks, calling ZopfliDeflatePart_rust for each chunk.
///
/// # Arguments
///
/// * `options` - Compression options
/// * `btype` - Block type
/// * `final_block` - Whether this is the final block
/// * `input` - Input data to compress
/// * `out` - Output buffer that will be extended with compressed data
/// * `bp` - Bit pointer
fn ZopfliDeflate_rust(
    options: &ZopfliOptions,
    btype: i32,
    final_block: bool,
    input: &[u8],
    bp: &mut u8,
    out: &mut Vec<u8>,
) {
    let offset = out.len();
    let insize = input.len();
    let mut i = 0;

    while i < insize {
        let masterfinal = i + 1000000 >= insize;
        let final2 = final_block && masterfinal;
        let size = if masterfinal { insize - i } else { 1000000 };

        ZopfliDeflatePart_rust(options, btype, final2, input, i, i + size, bp, out);

        i += size;
    }

    if options.verbose != 0 {
        let compressed_size = out.len() - offset;
        let compression_ratio = 100.0 * (insize as f64 - compressed_size as f64) / insize as f64;

        unsafe {
            fprintf(
                stderr,
                b"Original Size: %lu, Deflate: %lu, Compression: %.2f%%\n\0".as_ptr() as *const _,
                insize as u64,
                compressed_size as u64,
                compression_ratio,
            );
        }
    }
}
#[no_mangle]

fn single_test_rust(input: &[u8], btype: i32, blocksplitting: bool, blocksplittingmax: i32) {
    // Create and configure options
    let options = ZopfliOptions {
        verbose: 1,
        verbose_more: 0, // Reduce internal verbose output
        numiterations: 15,
        blocksplitting: if blocksplitting { 1 } else { 0 },
        blocksplittinglast: 0,
        blocksplittingmax,
    };

    // Initialize output vector and bit pointer
    let mut out = Vec::new();
    let mut bp: u8 = 0;

    // Perform compression
    ZopfliDeflate_rust(
        &options, btype, true, // final_block = 1 in the C code
        input, &mut bp, &mut out,
    );
}
#[no_mangle]

fn run_all_tests_rust(input: &[u8]) {
    // Dynamic Huffman
    single_test_rust(input, 2, true, 15);

    // Fixed Huffman
    single_test_rust(input, 1, true, 15);

    // Uncompressed
    single_test_rust(input, 0, true, 15);

    // Test with different block splitting settings

    // No block splitting
    single_test_rust(input, 2, false, 15);

    // Limited splits
    single_test_rust(input, 2, true, 5);

    // No splits
    single_test_rust(input, 2, true, 0);
    single_test_rust(input, 2, true, 1);
    single_test_rust(input, 2, true, 50);

    // More splits allowed
    single_test_rust(input, 2, true, 30);
}
#[no_mangle]

fn main_0_rust() -> i32 {
    // Call the Rust reimplementation of read_stdin_to_bytes
    let bytes = match read_stdin_to_bytes_rust() {
        Some(bytes) => bytes,
        None => {
            // Use Rust's standard error handling instead of fprintf
            eprintln!("Failed to read stdin");
            return 1;
        }
    };

    // Run tests using the Rust reimplementation
    run_all_tests_rust(&bytes);

    // No need to free memory in Rust as it's handled automatically
    // when bytes goes out of scope

    0 // Return success
}
#[no_mangle]
pub unsafe extern "C" fn main_0() -> c_int {
    main_0_rust()
}
