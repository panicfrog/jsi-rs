use std::{env, path::PathBuf};

fn main() {
    let base = env::var_os("CARGO_MANIFEST_DIR").unwrap();
    let pkg_base = PathBuf::from(base);

    let target_os = env::var("CARGO_CFG_TARGET_OS").ok();

    // Root of the react-native package (the one containing ReactCommon/).
    // Defaults to the React Native tree vendored by this branch.
    println!("cargo:rerun-if-env-changed=REACT_NATIVE_DIR");
    let rn_base = match env::var_os("REACT_NATIVE_DIR") {
        Some(dir) => {
            let dir = PathBuf::from(dir);
            if !dir.join("ReactCommon").is_dir() {
                panic!(
                    "\njsi-rn: REACT_NATIVE_DIR is set to `{}`, but it has no ReactCommon/.\n  \
                     Point it at the root of the react-native package, e.g.\n  \
                     <app>/node_modules/react-native\n",
                    dir.display()
                );
            }
            dir
        }
        None => {
            let vendored = pkg_base.join("vendor/react-native/packages/react-native");
            if !vendored.join("ReactCommon").is_dir() {
                panic!(
                    "\njsi-rn: could not find React Native.\n  \
                     This crate defaults to the tree vendored in the jsi-rs checkout, which\n  \
                     is missing — either run `git submodule update --init` there, or\n  \
                     (recommended for apps outside that repo) point REACT_NATIVE_DIR at\n  \
                     your app's copy:\n\n  \
                     # <app>/native/rust-app/.cargo/config.toml\n  \
                     [env]\n  \
                     REACT_NATIVE_DIR = {{ value = \"../../node_modules/react-native\", relative = true }}\n\n  \
                     See jsi-turbo/docs/usage.md for the full wiring.\n"
                );
            }
            vendored
        }
    };

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
        // fbjni headers: not shipped inside the react-native npm package.
        // Overridable for setups that source them elsewhere (e.g. a gradle
        // prefab cache); defaults to the vendored checkout.
        println!("cargo:rerun-if-env-changed=FBJNI_DIR");
        let fbjni_dir = match env::var_os("FBJNI_DIR") {
            Some(dir) => PathBuf::from(dir),
            None => {
                let vendored = pkg_base.join("vendor/fbjni/cxx");
                if !vendored.is_dir() {
                    panic!(
                        "\njsi-rn: Android builds need the fbjni headers, which are not part\n  \
                         of the react-native npm package. Either run\n  \
                         `git submodule update --init` in the jsi-rs checkout (default),\n  \
                         or set FBJNI_DIR to a directory containing fbjni/ headers\n  \
                         (e.g. from the com.facebook.fbjni:fbjni AAR's prefab include dir).\n"
                    );
                }
                vendored
            }
        };
        includes.push(fbjni_dir);
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
