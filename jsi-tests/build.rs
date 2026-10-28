use std::{env, path::PathBuf, process::Command};

fn main() {
    let pkg_base = env::var_os("CARGO_MANIFEST_DIR").unwrap();
    let pkg_base = PathBuf::from(pkg_base);

    let hermes_build_status = Command::new("bash")
        .args([pkg_base.join("../vendor/build-hermes.sh")])
        .current_dir(pkg_base.join("../vendor"))
        .output()
        .expect("hermes build script could not be executed");

    if !hermes_build_status.status.success() {
        panic!(
            "hermes build script failed\n\nstdout: {}\n\nstderr: {}",
            String::from_utf8_lossy(&hermes_build_status.stdout),
            String::from_utf8_lossy(&hermes_build_status.stderr),
        )
    }

    let rn_base = pkg_base.join("../vendor/react-native/packages/react-native");

    let includes = vec![
        rn_base.join("React"),
        rn_base.join("React/Base"),
        rn_base.join("ReactCommon/jsi"),
        rn_base.join("ReactCommon/callinvoker"),
        pkg_base.join("../vendor/hermes/API"),
        pkg_base.join("../vendor/hermes/public"),
        pkg_base.join("include"),
    ];

    for include in &includes {
        println!("cargo:rerun-if-changed={:?}", include);
    }

    let includes: Vec<_> = IntoIterator::into_iter(includes)
        .map(|p| dunce::canonicalize(&p).expect(&format!("missing include path {:?}", p)))
        .collect();

    let compiles: Vec<PathBuf> = vec![];

    let compiles: Vec<_> = IntoIterator::into_iter(compiles)
        .map(|p| dunce::canonicalize(&p).expect(&format!("missing compile file {:?}", p)))
        .collect();

    cxx_build::CFG
        .exported_header_dirs
        .extend(includes.iter().map(|e| e.as_path()));

    let bridges = vec!["src/ffi.rs"];

    for bridge in &bridges {
        println!("cargo:rerun-if-changed={}", bridge);
    }

    cxx_build::bridges(bridges)
        .flag_if_supported("-std=c++17")
        .files(compiles)
        .compile("js-tests");

    let hermes_lib_dir = pkg_base.join("../vendor/hermes/build/API/hermes/");

    // on macOS, hermes builds as a .framework bundle, so it must be linked and
    // found via -framework; on other platforms it's a plain shared library
    if hermes_lib_dir.join("hermes.framework").is_dir() {
        println!(
            "cargo:rustc-link-search=framework={}",
            hermes_lib_dir.to_string_lossy()
        );
        println!("cargo:rustc-link-lib=framework=hermes");
    } else {
        println!("cargo:rustc-link-lib=hermes");
        println!("cargo:rustc-link-search={}", hermes_lib_dir.to_string_lossy());
    }

    // the framework's install name is @rpath/..., so give test binaries an
    // rpath pointing at the hermes build output
    println!("cargo:rustc-link-arg=-Wl,-rpath,{}", hermes_lib_dir.to_string_lossy());
}
