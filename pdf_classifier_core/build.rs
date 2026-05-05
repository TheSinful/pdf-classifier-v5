use std::env;

fn main() {
    let root = env::var("CARGO_MANIFEST_DIR").unwrap();

    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=src/ffi.rs");
    println!("cargo:rerun-if-changed={}/pdf_classifier_ffi/ffi.cpp", root);
    println!("cargo:rerun-if-changed={}/pdf_classifier_ffi/ffi.hpp", root);
    println!(
        "cargo:rerun-if-changed={}/pdf_classifier_ffi/CMakeLists.txt",
        root
    );
    println!("cargo:rerun-if-env-changed=CXX");
    println!("cargo:rerun-if-env-changed=CC");
    println!("cargo:rerun-if-env-changed=CLASSIFIER_BUILD_DIR");

    let build_dir =
        env::var("CLASSIFIER_BUILD_DIR").unwrap_or(format!("{}/../examples/build", root));
    let mut build = cxx_build::bridges(&["src/ffi.rs"]);
    build
        .flag_if_supported("/std:c++20")
        .file("../pdf_classifier_ffi/ffi.cpp")
        .include(format!("{}/include", build_dir))
        .include("../pdf_classifier_ffi")
        .cpp(true)
        .compile("bindings");

    // Link all external libs (i.e MuPDF for bindings)
    println!("cargo:rustc-link-search=native={}/lib", build_dir);

    // link exported bindings.lib from cxx
    let out_dir = env::var("OUT_DIR").unwrap();
    println!("cargo:rustc-link-search=native={}", out_dir);

    println!("cargo:rustc-link-lib=static=libmupdf"); // may be an issue in the future see: https://github.com/TheSinful/pdf-classifier-v5/issues/1,  
    println!("cargo:rustc-link-lib=static=bindings");
    println!("cargo:rustc-link-lib=static=classifier_intermediary");
}
