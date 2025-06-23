extern crate bindgen;
extern crate serde_json;
extern crate glob;

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
// use std::process::Command;
// use glob::glob;

use bindgen::callbacks::{ParseCallbacks, ItemInfo, ItemKind};

#[derive(Debug)]
struct Renamer;

impl ParseCallbacks for Renamer {
    fn item_name(&self, original_item_name: &str) -> Option<String> {
        if original_item_name == "main" {
            return Some("main_0".to_string());
        }
        Some(original_item_name.to_string())
    }
    fn generated_link_name_override(&self, item_info: ItemInfo) -> Option<String> {
        if let ItemInfo{kind: ItemKind::Function, name, ..} = item_info {
            if name == "main" {
                return Some("main_0".to_string());
            }
        }
        Some(item_info.name.to_string())
    }
}

fn main() {
    let cargo_manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let c_build_path = env::var("C_BUILD_PATH").expect("C_BUILD_PATH not set");

    // Go through {c_build_path}/compile_commands.json, get the list of files
    let compile_commands_path = PathBuf::from(format!("{}/compile_commands.json", c_build_path));
    let compile_commands = std::fs::read_to_string(compile_commands_path)
        .expect("Unable to read compile_commands.json");
    let compile_commands: serde_json::Value = serde_json::from_str(&compile_commands)
        .expect("Unable to parse compile_commands.json");
    let files = compile_commands.as_array().expect("Expected an array");
    let source_paths = files.iter().map(|file| {
        let file = file.as_object().expect("Expected an object");
        let directory = file.get("directory").expect("Expected a directory").as_str().expect("Expected a string");
        let file_path = file.get("file").expect("Expected a file path").as_str().expect("Expected a string");
        // Check if the file path is absolute or relative
        // If it's relative, make it absolute by joining with the directory
        // If it's absolute, just use it as is
        // Create a PathBuf depending on whether the path is absolute
        let full_path = if Path::new(file_path).is_absolute() {
            PathBuf::from(file_path)
        } else {
            Path::new(directory).join(file_path)
        };
        full_path
    }).collect::<Vec<_>>();

    // // Expand "*.c" files in the src directory
    // let c_files: Vec<String> = glob(&format!("{}/*.c", c_build_path))
    //     .expect("Failed to read glob pattern")
    //     .filter_map(Result::ok) // Filter out errors
    //     .map(|path| path.display().to_string()) // Convert to string
    //     .collect();

    if source_paths.is_empty() {
        panic!("No .c files found in compile_commands.json");
    }

    let mut main_file: Option<String> = None;
    let mut main_num_args: i32 = 0;

    // Read c_build_path/analysis.json to find the function main_0
    let analysis_path = Path::new(&c_build_path).join("analysis.json");
    let analysis = std::fs::read_to_string(analysis_path)
        .expect("Unable to read analysis.json");
    let analysis: serde_json::Value = serde_json::from_str(&analysis)
        .expect("Unable to parse analysis.json");
    let functions = analysis.get("functions")
                            .expect("'functions' not found in json")
                            .as_array()
                            .expect("Expected 'functions' to be an array");
    for function in functions {
        let function = function.as_object().expect("Expected an object");
        let name = function.get("name").expect("Expected a name").as_str().expect("Expected a string");
        if name == "main_0" {
            let main_file_name = function.get("filename").expect("Expected a filename").as_str().expect("Expected a string");
            main_file = if Path::new(main_file_name).is_absolute() {
                Some(PathBuf::from(main_file_name).to_str().unwrap().to_string())
            } else {
                Some(Path::new(&c_build_path).join(main_file_name).to_str().unwrap().to_string())
            };
            main_num_args = function.get("num_args").expect("Expected a num_args").as_i64().expect("Expected an integer") as i32;
        }
    }

    // Tell cargo to tell rustc to link the library.
    println!("cargo::rustc-link-search=native={}", c_build_path);
    println!("cargo:rustc-link-lib=static=foo");

    // The bindgen::Builder is the main entry point
    // to bindgen, and lets you build up options for
    // the resulting bindings.
    let mut bindings = bindgen::Builder::default()
        .wrap_static_fns(false) // Rust linker won't find static functions because they don't have external linkage
        .generate_inline_functions(true)
        .parse_callbacks(Box::new(Renamer)); // Need to rename main as main_0

    if main_file.is_some() {
        // Add the main file to the builder
        // Assuming that the main file includes all the relevant header files
        bindings = bindings.header(main_file.unwrap());
    }
    else {
        // If we couldn't find a file with `main`, raise an error
        panic!("No main function found in C code");
        // bindings = source_paths.iter().fold(bindings, |bindings, file_path| {
        //     bindings.header(file_path.to_str().unwrap())
        // });
    }
    // Also for each entry in compile_commands, read the "arguments" list and look for "-I.."
    // and add them to the builder
    let mut include_paths = Vec::new();
    for file in files {
        let file = file.as_object().expect("Expected an object");
        let directory = file.get("directory").expect("Expected a directory").as_str().expect("Expected a string");
        let arguments = file.get("arguments").expect("Expected arguments").as_array().expect("Expected an array");
        for arg in arguments {
            if let Some(arg) = arg.as_str() {
                if arg.starts_with("-I") {
                    let include_path = arg[2..].trim_start().to_string();
                    let include_path = if Path::new(&include_path).is_absolute() {
                        PathBuf::from(&include_path)
                    } else {
                        Path::new(directory).join(&include_path)
                    };
                    // Convert the PathBuf to a string
                    let include_path = include_path.to_str().unwrap().to_string();
                    // Check if the include path is already in the list
                    // If not, add it to the list
                    if !include_paths.contains(&include_path) {
                        include_paths.push(include_path);
                    }
                }
            }
        }
    }
    // Add the include paths to the builder
    for include_path in include_paths {
        bindings = bindings.clang_arg(format!("-I{}", include_path));
    }

    // Read a list of blocklist functions from bindgen_blocklist.txt
    // If the file doesn't exist, we'll just use an empty blocklist
    let blocklist: Vec<String> = match std::fs::read_to_string("bindgen_blocklist.txt") {
        Ok(blocklist) => {
            blocklist.lines().map(String::from).collect()
        },
        Err(_) => Vec::new(),
    };
    
    let bindings = blocklist.iter().fold(bindings, |bindings, function| {
        if function == "main_0" {
            bindings.blocklist_function("main")
        }
        else {
            bindings.blocklist_function(function)
        }
    });
    // Bindgen uses `i` in loops, and this creates conflicts if there is a global variable named `i` in the C code.
    // This is not perfect, so it needs fixing in the long term.
    let bindings = bindings.blocklist_item("i");

    let bindings = bindings.generate() // Finish the builder and generate the bindings.
                        .expect("Unable to generate bindings"); // Unwrap the Result and panic on failure.
    
    // Create main_func.rs
    let main_file = PathBuf::from(format!("{}/src/main_func.rs", cargo_manifest_dir));
    let main_file_content: String = if main_num_args == 2 {
        format!(r#"
pub fn main() {{
    let mut args: Vec<*mut libc::c_char> = Vec::new();
    for arg in ::std::env::args() {{
        args.push(
            (::std::ffi::CString::new(arg))
                .expect("Failed to convert argument into CString.")
                .into_raw(),
        );
    }}
    args.push(::core::ptr::null_mut());
    unsafe {{
        ::std::process::exit(main_0(
            (args.len() - 1) as libc::c_int,
            args.as_mut_ptr() as *mut *mut libc::c_char,
        ) as i32)
    }}
}}"#)
    } else if main_num_args == 0 {
        format!(r#"
pub fn main() {{
    unsafe {{
        ::std::process::exit(main_0() as i32)
    }}
}}"#)
    } else {
        // Raise error if main_num_args is not 0 or 2
        panic!("main_0 has {} arguments, expected 0 or 2", main_num_args);
    };
    // Write the main file to disk
    fs::write(main_file, main_file_content)
        .expect("Unable to write main_func.rs");

    // Write the bindings to the $OUT_DIR/bindings.rs file.
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");
}