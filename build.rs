use std::env;

fn main() {
    cxx_build::bridges(&["src/initializer.rs"])
        .flag_if_supported("/std:c++20")
        .file("src_cpp/initializer.cpp")
        .include("./src_cpp")
        .include("./include")
        .cpp(true)
        .compile("bindings");

    // Link all external libs (i.e MuPDF for bindings)
    let root = env::var("CARGO_MANIFEST_DIR").unwrap();
    println!("cargo:rustc-link-search=native={}/lib", root);

    // link exported bindings.lib from cxx
    let out_dir = env::var("OUT_DIR").unwrap();
    println!("cargo:rustc-link-search=native={}", out_dir);
    println!("cargo:rustc-link-lib=static=bindings");
}
