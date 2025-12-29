use std::env;

fn main() {
    cxx_build::bridges(&["src/ffi.rs"])
        .flag_if_supported("/std:c++20")
        .file("src_cpp/initializer.cpp")
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
    println!("cargo:rerun-if-changed=src_cpp/initializer.cpp");
    println!("cargo:rerun-if-changed=CMakeLists.txt");
}
