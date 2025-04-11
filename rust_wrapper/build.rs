extern crate bindgen;
extern crate serde_json;
extern crate glob;

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use glob::glob;

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
    let c_src_dir = format!("{}/c_src", cargo_manifest_dir);
    
    // Read "PARSEC_BUILD_DIR" from the environment
    let parsec_build_dir = env::var("PARSEC_BUILD_DIR").expect("PARSEC_BUILD_DIR not set");
    // Convert PARSEC_BUILD_DIR to absolute path if it's relative
    let parsec_build_dir = fs::canonicalize(Path::new(&cargo_manifest_dir).join(&parsec_build_dir))
                                .expect("Failed to resolve absolute path for PARSEC_BUILD_DIR");
    let new_path = format!("{}:{}", parsec_build_dir.display(), env::var("PATH").unwrap_or_default());
    env::set_var("PATH", new_path);

    // Expand "*.c" files in the src directory
    let c_files: Vec<String> = glob(&format!("{}/*.c", c_src_dir))
        .expect("Failed to read glob pattern")
        .filter_map(Result::ok) // Filter out errors
        .map(|path| path.display().to_string()) // Convert to string
        .collect();

    if c_files.is_empty() {
        panic!("No .c files found in {}", c_src_dir);
    }

    // First run parsec to generate libfoo.a
    let status = Command::new("parsec")
        .current_dir(&c_src_dir)
        .args(&c_files)
        .status()
        .expect("Failed to run parsec");
    
    if !status.success() {
        panic!("Parsec failed with exit code: {:?}", status.code());
    }

    // Tell cargo to tell rustc to link the library.
    println!("cargo::rustc-link-search=native=c_src");
    println!("cargo:rustc-link-lib=static=foo");

    // The bindgen::Builder is the main entry point
    // to bindgen, and lets you build up options for
    // the resulting bindings.
    let bindings = bindgen::Builder::default()
        // The input header we would like to generate
        // bindings for.
        .wrap_static_fns(true)
        .generate_inline_functions(true)
        .parse_callbacks(Box::new(Renamer)); // Need to rename main as main_0
    
    // Go through c_src/compile_commands.json, get the list of files
    // and add all of them to the builder
    let compile_commands_path = PathBuf::from("c_src/compile_commands.json");
    let compile_commands = std::fs::read_to_string(compile_commands_path)
        .expect("Unable to read compile_commands.json");
    let compile_commands: serde_json::Value = serde_json::from_str(&compile_commands)
        .expect("Unable to parse compile_commands.json");
    let files = compile_commands.as_array().expect("Expected an array");
    let mut bindings = files.iter().fold(bindings, |bindings, file| {
        let file = file.as_object().expect("Expected an object");
        let file_path = file.get("file").expect("Expected a file path").as_str().expect("Expected a string");
        let file_path = PathBuf::from(file_path);
        bindings.header(file_path.to_str().unwrap())
    });
    // Also for each entry in compile_commands, read the "arguments" list and look for "-I.."
    // and add them to the builder
    let mut include_paths = Vec::new();
    for file in files {
        let file = file.as_object().expect("Expected an object");
        let arguments = file.get("arguments").expect("Expected arguments").as_array().expect("Expected an array");
        for arg in arguments {
            if let Some(arg) = arg.as_str() {
                if arg.starts_with("-I") {
                    let include_path = arg[2..].trim_start().to_string();
                    include_paths.push(include_path);
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
    

    let bindings = bindings.generate() // Finish the builder and generate the bindings.
                        .expect("Unable to generate bindings"); // Unwrap the Result and panic on failure.

    // Write the bindings to the $OUT_DIR/bindings.rs file.
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");
}