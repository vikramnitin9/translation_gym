#![feature(backtrace)]
#![feature(rustc_private)]
#![feature(let_else)]
#![feature(min_specialization)]
#![feature(array_windows)]
#![feature(once_cell)]

extern crate rustc_driver;
extern crate rustc_interface;
extern crate rustc_hir;
extern crate rustc_middle;
extern crate rustc_span;
extern crate rustc_version;
extern crate rustc_mir_dataflow;
extern crate rustc_data_structures;
extern crate rustc_index;
extern crate rustc_serialize;

extern crate smallvec;
extern crate cargo_metadata;
extern crate rand;
extern crate itertools;

use core::panic;
use std::io::Write;

use rustc_middle::ty::TyCtxt;
use rustc_span::Span;

mod utils;
mod unsafe_counter;
mod pointer_counter;
mod laertes_helper;

use utils::*;
use unsafe_counter::UnsafeCounter;
use pointer_counter::PointerCounter;
use rustc_span::def_id::LOCAL_CRATE;

#[derive(Debug, Clone)]
pub struct MetricsConfig {
    pub selected_fns_file: String,
}

impl Default for MetricsConfig {
    fn default() -> Self {
        MetricsConfig {
            selected_fns_file: "".to_string(),
        }
    }
}

/// Returns the "default sysroot" that Metrics will use if no `--sysroot` flag is set.
/// Should be a compile-time constant.
pub fn compile_time_sysroot() -> Option<String> {
    // option_env! is replaced to a constant at compile time
    if option_env!("RUSTC_STAGE").is_some() {
        // This is being built as part of rustc, and gets shipped with rustup.
        // We can rely on the sysroot computation in librustc.
        return None;
    }

    // For builds outside rustc, we need to ensure that we got a sysroot
    // that gets used as a default. The sysroot computation in librustc would
    // end up somewhere in the build dir.
    // Taken from PR <https://github.com/Manishearth/rust-clippy/pull/911>.
    let home = option_env!("RUSTUP_HOME").or(option_env!("MULTIRUST_HOME"));
    let toolchain = option_env!("RUSTUP_TOOLCHAIN").or(option_env!("MULTIRUST_TOOLCHAIN"));
    Some(match (home, toolchain) {
        (Some(home), Some(toolchain)) => format!("{}/toolchains/{}", home, toolchain),
        _ => option_env!("RUST_SYSROOT")
            .expect("To build Metrics without rustup, set the `RUST_SYSROOT` env var at build time")
            .to_owned(),
    })
}

pub fn count_file_spans(tcx: TyCtxt<'_>, crate_num: rustc_span::def_id::CrateNum) -> u32 {
    let hir_map = tcx.hir();
    let source_map = tcx.sess.source_map();
    let mut bytes_count: u32 = 0;
    let files = &*(source_map.files());
    for source_file in files.iter() {
        let rustc_span::FileName::Real(rustc_span::RealFileName::LocalPath(filename)) = &source_file.name else {
            continue;
        };
        if source_file.cnum != crate_num {
            continue;
        }
        let length = source_file.source_len.0;
        bytes_count += length;
    }
    bytes_count
}

pub fn add_span_lengths(spans: &Vec<Span>, tcx: TyCtxt<'_>) -> u32 {
    // spans.iter().map(|span| get_span_length(tcx, span)).sum()
    spans.iter().map(|span| get_span_lines(tcx, span) as u32).sum()

}

pub fn analyze(&tcx: &TyCtxt<'_>, config: MetricsConfig) {

    let selected_fns: Vec<String> = if config.selected_fns_file == "" {
        Vec::new()
    } else {
        match std::fs::read_to_string(&config.selected_fns_file) {
            Ok(selected_fns) => {
                selected_fns.lines().map(String::from).collect()
            },
            Err(_) => {
                panic!("Could not read selected functions file");
            },
        }
    };

    let hir_map = tcx.hir();
    let source_map = tcx.sess.source_map();
    let mut unsafe_counter = UnsafeCounter::new(&tcx, selected_fns.clone());
    tcx.hir().visit_all_item_likes_in_crate(&mut unsafe_counter);

    let total_unsafe_spans = add_span_lengths(&unsafe_counter.unsafe_spans, tcx);
    let total_file_spans = count_file_spans(tcx, LOCAL_CRATE);
    let total_unsafe_calls = unsafe_counter.unsafe_calls.len();
    let total_unsafe_casts = unsafe_counter.unsafe_casts.len();

    println!("Unsafe lines: {}", total_unsafe_spans);
    println!("Unsafe calls: {}", total_unsafe_calls);
    println!("Unsafe casts: {}", total_unsafe_casts);
    // println!("Total spans: {}", total_file_spans);
    // println!("Unsafe ratio: {}", total_unsafe_spans as f64 / total_file_spans as f64);

    // Append all derefs to an existing file unsafe_spans.txt
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .append(true)
        .open("unsafe_spans.txt")
        .unwrap();
    for span in &unsafe_counter.unsafe_spans {
        let diagnostic_str = span_to_diagnostic(tcx, span);
        // let span_str = span_to_string(tcx, span);
        file.write_all(diagnostic_str.as_bytes()).unwrap();
        file.write(b"\n").unwrap();
        // file.write_all(span_str.as_bytes()).unwrap();
        // file.write(b"\n").unwrap();
    }

    let mut pointer_counter = PointerCounter::new(&tcx, selected_fns.clone());
    tcx.hir().visit_all_item_likes_in_crate(&mut pointer_counter);

    println!("Raw pointer dereferences: {}", pointer_counter.all_derefs.len());
    println!("Raw pointer declarations: {}", pointer_counter.all_decls.len());

    // Append all derefs to an existing file derefs.txt
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .append(true)
        .open("derefs.txt")
        .unwrap();

    for span in &pointer_counter.all_derefs {
        let diagnostic_str = span_to_diagnostic(tcx, span);
        // let span_str = span_to_string(tcx, span);
        file.write_all(diagnostic_str.as_bytes()).unwrap();
        file.write(b"\n").unwrap();
        // file.write_all(span_str.as_bytes()).unwrap();
        // file.write(b"\n").unwrap();
    }

    // Append all derefs to an existing file derefs.txt
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .append(true)
        .open("decls.txt")
        .unwrap();
    for span in &pointer_counter.all_decls {
        let diagnostic_str = span_to_diagnostic(tcx, span);
        // let span_str = span_to_string(tcx, span);
        file.write_all(diagnostic_str.as_bytes()).unwrap();
        file.write(b"\n").unwrap();
        // file.write_all(span_str.as_bytes()).unwrap();
        // file.write(b"\n").unwrap();
    }
}