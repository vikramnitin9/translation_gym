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
            output_file: "functions.json".to_string(),
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

pub fn analyze(&tcx: &TyCtxt<'_>, config: ParseConfig) {

    let mut visitor = visitor::CallgraphVisitor::new(&tcx);
    tcx.hir().visit_all_item_likes_in_crate(&mut visitor);

    let mut json_output: Vec<serde_json::Value> = vec![];

    for (func_defid, _) in visitor.functions.iter() {

        let func_info = visitor.functions.get(func_defid).unwrap();
        let ((fname, start_line, start_col), (_, end_line, end_col)) = span_to_data(tcx, &func_info.span);

        let mut this_func_json = json!({
            "name": tcx.item_name(*func_defid).to_string(),
            "filename": fname,
            "signature": span_to_string(tcx, &func_info.sig_span),
            "startLine": start_line,
            "startCol": start_col,
            "endLine": end_line,
            "endCol": end_col,
            "calls": [],
            "globals": [],
            "imports": [],
            "foreign": func_info.foreign,
        });

        // Get all the imports in the same file as the function definition
        let imports = get_imports_from_same_file(&func_info.span, &visitor.imports, &tcx);
        if imports.len() > 0 {
            for import in imports {
                let import_json = json!({
                    "span": format!("{:?}", import),
                    "source": span_to_string(tcx, &import)
                });
                this_func_json["imports"].as_array_mut().unwrap().push(import_json);
            }
        }
        // Get all the calls to this function
        for call in visitor.static_calls.iter().chain(visitor.dynamic_calls.iter()) {
            if call.callee == *func_defid {
                let call_span = get_parent_span(&call.call_expr, &tcx).unwrap();
                let call_json = json!({
                    "caller": if call.caller.is_some() { format!("{:?}", call.caller.unwrap()) } else { "".to_string()},
                    "span": format!("{:?}", call_span),
                    "source": span_to_string(tcx, &call_span)
                });
                this_func_json["calls"].as_array_mut().unwrap().push(call_json);
            }
        }
        // Get globals corresponding to this function
        if let Some(globals) = visitor.globals.get(func_defid) {
            if globals.len() > 0 {
                for global in globals {
                    let global_span = global;
                    let global_json = json!({
                        "span": format!("{:?}", global_span),
                        "source": span_to_string(tcx, &global_span)
                    });
                    this_func_json["globals"].as_array_mut().unwrap().push(global_json);
                }
            }
        }
        json_output.push(this_func_json);
    }
    
    // Write the json to a file
    let stringified_json = serde_json::to_string_pretty(&json_output).unwrap();
    std::fs::write(config.clone().output_file, stringified_json).unwrap();
}