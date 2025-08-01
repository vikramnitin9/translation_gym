#![feature(rustc_private)]
#![feature(min_specialization)]
#![feature(array_windows)]

extern crate rustc_driver;
extern crate rustc_interface;
extern crate rustc_hir;
extern crate rustc_middle;
extern crate rustc_span;
extern crate rustc_mir_dataflow;
extern crate rustc_data_structures;
extern crate rustc_index;
extern crate rustc_serialize;

extern crate rustc_version;
extern crate cargo_metadata;
extern crate petgraph;
extern crate smallvec;
extern crate itertools;
extern crate regex;

use std::collections::HashSet;
use rustc_middle::ty::TyCtxt;
use serde_json::json;

mod visitor;
mod utils;

use utils::*;

#[derive(Debug, Clone)]
pub struct ParseConfig {
    pub output_file: String,
}

impl Default for ParseConfig {
    fn default() -> Self {
        ParseConfig {
            output_file: "analysis.json".to_string(),
        }
    }
}

/// Returns the "default sysroot" that Parse will use if no `--sysroot` flag is set.
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
            .expect("To build Parse without rustup, set the `RUST_SYSROOT` env var at build time")
            .to_owned(),
    })
}

pub fn get_fn_name(def_id: rustc_hir::def_id::DefId,
                   impl_id: Option<rustc_hir::def_id::DefId>,
                   tcx: TyCtxt<'_>) -> String {
    // Get the function name from the DefId
    let fn_name = tcx.item_name(def_id).to_string();
    let fn_name = if fn_name.is_empty() {
        // If the name is empty, "<unknown>"
        "<unknown>".to_string()
    } else {
        fn_name
    };
    // If the function is in an impl, prepend the impl name
    if let Some(impl_id) = impl_id {
        let impl_name = tcx.opt_item_name(impl_id);
        if let Some(impl_name) = impl_name {
            format!("{}::{}", impl_name.to_string(), fn_name)
        } else {
            fn_name
        }
    } else {
        fn_name
    }
}

pub fn analyze(&tcx: &TyCtxt<'_>, config: ParseConfig) {

    let mut visitor = visitor::CallgraphVisitor::new(&tcx);
    tcx.hir().visit_all_item_likes_in_crate(&mut visitor);

    let mut files_json: Vec<serde_json::Value> = vec![];
    let mut funcs_json: Vec<serde_json::Value> = vec![];
    let mut globals_json: Vec<serde_json::Value> = vec![];
    let mut structs_json: Vec<serde_json::Value> = vec![];
    let mut enums_json: Vec<serde_json::Value> = vec![];

    for file in visitor.files.iter() {
        let fname = file.to_str().unwrap_or("<unknown>").to_string();
        let mut file_imports = HashSet::<serde_json::Value>::new();
        for import in visitor.imports.iter() {
            let ((import_fname, _, _), (_, _, _)) = span_to_data(tcx, &import);
            let source = span_to_string(tcx, &import);
            if (import_fname == fname) && source.contains("use ") {
                file_imports.insert(json!({
                    "span": format!("{:?}", import),
                    "source": source,
                }));
            }
        }
        let file_imports_vec: Vec<serde_json::Value> = file_imports.into_iter().collect();
        files_json.push(json!({
            "filename": fname,
            "imports": file_imports_vec,
        }));
    }

    for (func_defid, _) in visitor.functions.iter() {

        let func_info = visitor.functions.get(func_defid).unwrap();
        let ((fname, start_line, start_col), (_, end_line, end_col)) = span_to_data(tcx, &func_info.span);

        let mut this_func_json = json!({
            "name": get_fn_name(*func_defid, func_info.impl_id, tcx),
            "filename": fname,
            "signature": span_to_string(tcx, &func_info.sig_span),
            "startLine": start_line,
            "startCol": start_col,
            "endLine": end_line,
            "endCol": end_col,
            "functions": [],
            "globals": [],
            "structs": [],
            "enums": [],
            "foreign": func_info.foreign,
        });

        // Get all functions called by this function
        for call in visitor.static_calls.iter().chain(visitor.dynamic_calls.iter()) {
            if call.caller == Some(*func_defid) {
                let impl_id = if let Some(callee_info) = visitor.functions.get(&call.callee) {
                    callee_info.impl_id
                } else {
                    None
                };
                let call_json = json!({
                    "name": get_fn_name(call.callee, impl_id, tcx),
                });
                this_func_json["functions"].as_array_mut().unwrap().push(call_json);
            }
        }
        // Get globals corresponding to this function
        for (global_def_id, global_span) in func_info.globals.iter() {
            let global_json = json!({
                "name": tcx.item_name(*global_def_id).to_string(),
            });
            this_func_json["globals"].as_array_mut().unwrap().push(global_json);
        }
        for (struct_defid, struct_span) in func_info.structs.iter() {
            let struct_json = json!({
                "name": tcx.item_name(*struct_defid).to_string(),
            });
            this_func_json["structs"].as_array_mut().unwrap().push(struct_json);
        }
        for (enum_defid, enum_span) in func_info.enums.iter() {
            let enum_json = json!({
                "name": tcx.item_name(*enum_defid).to_string(),
            });
            this_func_json["enums"].as_array_mut().unwrap().push(enum_json);
        }
        funcs_json.push(this_func_json);
    }

    for (global_defid, global_span) in visitor.globals.iter() {
        let ((fname, start_line, start_col), (_, end_line, end_col)) = span_to_data(tcx, &global_span);
        let foreign = fname.ends_with("bindings.rs"); // This is a quick fix for now but needs a better solution
        let global_json = json!({
            "name": tcx.item_name(*global_defid).to_string(),
            "filename": fname,
            "startLine": start_line,
            "startCol": start_col,
            "endLine": end_line,
            "endCol": end_col,
            "foreign": foreign,
        });
        globals_json.push(global_json);
    }

    for (struct_defid, struct_span) in visitor.structs.iter() {
        let ((fname, start_line, start_col), (_, end_line, end_col)) = span_to_data(tcx, &struct_span);
        let foreign = fname.ends_with("bindings.rs"); // This is a quick fix for now but needs a better solution
        let struct_json = json!({
            "name": tcx.item_name(*struct_defid).to_string(),
            "filename": fname,
            "startLine": start_line,
            "startCol": start_col,
            "endLine": end_line,
            "endCol": end_col,
            "foreign": foreign,
        });
        structs_json.push(struct_json);
    }

    for (enum_defid, enum_span) in visitor.enums.iter() {
        let ((fname, start_line, start_col), (_, end_line, end_col)) = span_to_data(tcx, &enum_span);
        let foreign = fname.ends_with("bindings.rs"); // This is a quick fix for now but needs a better solution
        let enum_json = json!({
            "name": tcx.item_name(*enum_defid).to_string(),
            "filename": fname,
            "startLine": start_line,
            "startCol": start_col,
            "endLine": end_line,
            "endCol": end_col,
            "foreign": foreign,
        });
        enums_json.push(enum_json);
    }
    
    let json_output = json!({
        "files": files_json,
        "functions": funcs_json,
        "globals": globals_json,
        "structs": structs_json,
        "enums": enums_json,
    });
    // Write the json to a file
    let stringified_json = serde_json::to_string_pretty(&json_output).unwrap();
    std::fs::write(config.clone().output_file, stringified_json).unwrap();
}