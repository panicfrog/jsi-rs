use std::{env, path::PathBuf};

fn main() {
    let base = env::var_os("CARGO_MANIFEST_DIR").unwrap();
    let pkg_base = PathBuf::from(base);

    // Directory containing `jsi/jsi.h` (and `jsi/jsi.cpp` when the
    // `bundled-jsi` feature is enabled). jsi is engine-agnostic, so any copy of
    // it works, as long as it is the same version the JS engine was built
    // against: it is the ABI contract between host and engine. Defaults to the
    // copy in the vendored React Native tree that this branch is paired with.
    println!("cargo:rerun-if-env-changed=JSI_DIR");
    let jsi_dir = match env::var_os("JSI_DIR") {
        Some(dir) => {
            let dir = PathBuf::from(dir);
            if !dir.join("jsi/jsi.h").exists() {
                panic!(
                    "\njsi-sys: JSI_DIR is set to `{}`, but `jsi/jsi.h` was not found there.\n  \
                     Point it at a directory containing jsi/jsi.h, e.g. your app's\n  \
                     <app>/node_modules/react-native/ReactCommon/jsi\n",
                    dir.display()
                );
            }
            dir
        }
        None => {
            let vendored = pkg_base
                .join("vendor/react-native/packages/react-native/ReactCommon/jsi");
            if !vendored.join("jsi/jsi.h").exists() {
                panic!(
                    "\njsi-sys: could not find jsi/jsi.h.\n  \
                     This crate defaults to the React Native tree vendored in the jsi-rs\n  \
                     checkout, which is missing — either run `git submodule update --init`\n  \
                     there, or (recommended for apps outside that repo) point JSI_DIR at\n  \
                     your app's copy:\n\n  \
                     # <app>/native/rust-app/.cargo/config.toml\n  \
                     [env]\n  \
                     JSI_DIR = {{ value = \"../../node_modules/react-native/ReactCommon/jsi\", relative = true }}\n\n  \
                     See jsi-turbo/docs/local-dependencies.md for the full wiring.\n"
                );
            }
            vendored
        }
    };

    let includes: Vec<_> = IntoIterator::into_iter(vec![jsi_dir.clone(), pkg_base.join("include")])
        .map(|p| dunce::canonicalize(&p).expect(&format!("missing include path {:?}", p)))
        .collect();

    // jsi.cpp holds the out-of-line parts of the jsi classes. Hosts that
    // already ship jsi (e.g. a React Native app) must provide those symbols
    // themselves, otherwise the program ends up with two copies of them.
    let mut compiles = vec![];
    if env::var_os("CARGO_FEATURE_BUNDLED_JSI").is_some() {
        compiles.push(jsi_dir.join("jsi/jsi.cpp"));
    }

    let compiles: Vec<_> = IntoIterator::into_iter(compiles)
        .map(|p| dunce::canonicalize(&p).expect(&format!("missing compile file {:?}", p)))
        .collect();

    cxx_build::CFG
        .exported_header_dirs
        .extend(includes.iter().map(|e| e.as_path()));

    let bridges = vec!["src/ffi/base.rs", "src/ffi/host.rs", "src/ffi/native_state.rs"];

    for bridge in &bridges {
        println!("cargo:rerun-if-changed={}", bridge);
    }

    cxx_build::bridges(bridges)
        .flag_if_supported("-std=c++17")
        .files(compiles)
        .compile("jsi");

    println!("cargo:rerun-if-changed=include/wrapper.h");
    println!("cargo:rerun-if-changed=include/host.h");
    println!("cargo:rerun-if-changed=include/native_state.h");
}
