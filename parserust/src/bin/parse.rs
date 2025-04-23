#![feature(rustc_private)]

extern crate rustc_driver;
extern crate rustc_interface;

use rustc_driver::{Callbacks, Compilation};
use rustc_interface::Queries;
use rustc_interface::interface::Compiler;

use parse::{analyze, compile_time_sysroot, ParseConfig};

pub static PARSE_DEFAULT_ARGS: &[&str] =
    &["-Zmir-emit-retag", "-Zalways-encode-mir", "-Zmir-opt-level=0"];

struct ParseCallbacks {
    config: ParseConfig,
}

impl ParseCallbacks {
    fn new(config: ParseConfig) -> Self {
        Self {
            config,
        }
    }
}

impl Callbacks for ParseCallbacks {

    fn after_analysis<'tcx>(&mut self, _compiler: &Compiler, queries: &'tcx Queries<'tcx>) -> Compilation {
        
        queries.global_ctxt().unwrap().enter(|tcx| {
            analyze(&tcx, self.config.clone());
        });

        Compilation::Continue
    }
}

fn parse_config() -> (ParseConfig, Vec<String>) {
    // collect arguments
    let mut config = ParseConfig::default();

    let mut rustc_args = vec![];
    let mut args = std::env::args();

    while let Some(arg) = args.next() {
        let orig_arg = arg.clone();
        let (key, value) = match arg.contains('=') {
            true => {
                let str_vec: Vec<&str> = arg.split('=').collect();
                (String::from(str_vec[0]), Some(String::from(str_vec[1])))
            },
            false => {
                (arg, None)
            }
        };
        match &key[..] {
            "-Zoutput-file" => {
                config.output_file = value.expect("Missing argument for -Zoutput-file");
            },
            _ => {
                rustc_args.push(orig_arg);
            }
        }
    }

    (config, rustc_args)
}

fn main() {

    let (config, mut rustc_args) = parse_config();

    // Make sure we use the right default sysroot. The default sysroot is wrong,
    // because `get_or_default_sysroot` in `librustc_session` bases that on `current_exe`.
    //
    // Make sure we always call `compile_time_sysroot` as that also does some sanity-checks
    // of the environment we were built in.
    // FIXME: Ideally we'd turn a bad build env into a compile-time error via CTFE or so.
    if let Some(sysroot) = compile_time_sysroot() {
        let sysroot_flag = "--sysroot";
        if !rustc_args.iter().any(|e| e == sysroot_flag) {
            // We need to overwrite the default that librustc_session would compute.
            rustc_args.push(sysroot_flag.to_owned());
            rustc_args.push(sysroot);
        }
    }

    rustc_args.splice(
        1..1,
        PARSE_DEFAULT_ARGS.iter().map(ToString::to_string),
    );

    let mut calls = ParseCallbacks::new(config);

    let exit_code = rustc_driver::catch_with_exit_code(move || {
        rustc_driver::RunCompiler::new(&rustc_args, &mut calls).run()
    });
    std::process::exit(exit_code)
}