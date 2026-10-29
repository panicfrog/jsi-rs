use std::{env, path::PathBuf};

fn main() {
    let base = env::var_os("CARGO_MANIFEST_DIR").unwrap();
    let pkg_base = PathBuf::from(base);

    let target_os = env::var("CARGO_CFG_TARGET_OS").ok();

    // Root of the react-native package (the one containing ReactCommon/).
    // Defaults to the React Native tree vendored by this branch.
    println!("cargo:rerun-if-env-changed=REACT_NATIVE_DIR");
    let rn_base = env::var_os("REACT_NATIVE_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| pkg_base.join("vendor/react-native/packages/react-native"));

    let mut includes = vec![
        rn_base.join("ReactCommon/callinvoker"),
        // CallInvoker.h includes react/cxxstableapi/UmbrellaGuard.h (RN 0.88+)
        rn_base.join("ReactCommon"),
        pkg_base.join("include"),
    ];
    let mut compiles = vec![];
    let mut bridges = vec!["src/ffi.rs"];

    if target_os.as_deref() == Some("android") {
        includes.push(rn_base.join("ReactAndroid/src/main/jni/react/turbomodule"));
        includes.push(pkg_base.join("vendor/fbjni/cxx"));
        compiles.push(rn_base.join(
            "ReactAndroid/src/main/jni/react/turbomodule/ReactCommon/CallInvokerHolder.cpp",
        ));
        bridges.push("src/android.rs");
    }

    let includes: Vec<_> = includes
        .into_iter()
        .map(|p| dunce::canonicalize(&p).expect(&format!("missing include path {:?}", p)))
        .collect();
    let compiles: Vec<_> = compiles
        .into_iter()
        .map(|p| dunce::canonicalize(&p).expect(&format!("missing compile file {:?}", p)))
        .collect();

    cxx_build::CFG
        .exported_header_dirs
        .extend(includes.iter().map(|e| e.as_path()));

    for bridge in &bridges {
        println!("cargo:rerun-if-changed={}", bridge);
    }

    cxx_build::bridges(bridges)
        .flag_if_supported("-std=c++17")
        .files(compiles)
        .compile("jsi-rn");

    println!("cargo:rerun-if-changed=include/rn_wrapper.h");
}
