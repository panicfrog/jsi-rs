fn main() {
    // The bridge re-uses jsi-sys' exported headers (wrapper.h and the
    // vendored RN includes), so no extra include dirs are needed here.
    let bridges = vec!["src/lib.rs"];
    for bridge in &bridges {
        println!("cargo:rerun-if-changed={}", bridge);
    }

    // iOS and Android both consume the staticlib: the archive keeps the cxx
    // bridge's C++ side (wrappers + runtime) available to the platform
    // linker, which pulls in whatever the glue references.
    cxx_build::bridges(bridges)
        .flag_if_supported("-std=c++17")
        .compile("rust-app");
}
