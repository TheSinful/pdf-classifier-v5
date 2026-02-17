use std::env;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=src_cpp");
    println!("cargo:rerun-if-changed=src/ffi.rs");
    println!("cargo:rerun-if-changed=src_cpp/ffi.cpp");
    println!("cargo:rerun-if-changed=CMakeLists.txt");
    println!("cargo:rerun-if-env-changed=CXX");
    println!("cargo:rerun-if-env-changed=CC");
    println!("cargo:rerun-if-env-changed=PROFILE");

    let mut build = cxx_build::bridges(&["src/ffi.rs"]);
    build
        .flag_if_supported("/std:c++20")
        .file("src_cpp/ffi.cpp")
        .include("./src_cpp")
        .include("./build/include")
        .cpp(true)
        .compile("bindings");

    // Link all external libs (i.e MuPDF for bindings)
    let root = env::var("CARGO_MANIFEST_DIR").unwrap();
    println!("cargo:rustc-link-search=native={}/build/lib", root);

    // link exported bindings.lib from cxx
    let out_dir = env::var("OUT_DIR").unwrap();
    println!("cargo:rustc-link-search=native={}", out_dir);

    println!("cargo:rustc-link-lib=static=libmupdf"); // may be an issue in the future see: https://github.com/TheSinful/pdf-classifier-v5/issues/1,  
    println!("cargo:rustc-link-lib=static=bindings");
    println!("cargo:rustc-link-lib=static=classifier_intermediary");
}
