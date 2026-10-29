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

    // same jsi copy as jsi-sys (see jsi-sys/build.rs)
    println!("cargo:rerun-if-env-changed=JSI_DIR");
    let jsi_dir = env::var_os("JSI_DIR").map(PathBuf::from).unwrap_or_else(|| {
        pkg_base.join("../vendor/react-native/packages/react-native/ReactCommon/jsi")
    });

    // only jsi and the hermes API: the tests exercise the engine-agnostic jsi
    // crate and must not depend on React Native
    let includes = vec![
        jsi_dir,
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

    let hermes_api_dir = pkg_base.join("../vendor/hermes/build/API/hermes/");
    let _hermes_lib_dir = pkg_base.join("../vendor/hermes/build/lib/");

    if hermes_api_dir.join("hermes.framework").is_dir() {
        // hermes up to RN 0.77: a single .framework bundle on macOS
        println!(
            "cargo:rustc-link-search=framework={}",
            hermes_api_dir.to_string_lossy()
        );
        println!("cargo:rustc-link-lib=framework=hermes");
        println!(
            "cargo:rustc-link-arg=-Wl,-rpath,{}",
            hermes_api_dir.to_string_lossy()
        );
    } else if hermes_api_dir.join("libhermesapi.a").is_file() {
        // hermes from RN 0.88 builds static libraries only; mirror the link
        // line of the hermes CLI itself (see `ninja -t commands hermes`),
        // minus napi and hermes' own jsi (jsi-sys provides those symbols)
        let build_dir = pkg_base.join("../vendor/hermes/build/");
        for dir in [
            &hermes_api_dir,
            &build_dir.join("lib/"),
            &build_dir.join("lib/VM/"),
            &build_dir.join("public/hermes/Public/"),
            &build_dir.join("external/boost/boost_1_86_0/libs/context/"),
        ] {
            println!("cargo:rustc-link-search=native={}", dir.to_string_lossy());
        }
        println!("cargo:rustc-link-lib=static=hermesvm_a");
        println!("cargo:rustc-link-lib=static=hermesVMRuntime");
        println!("cargo:rustc-link-lib=static=hermesapi");
        println!("cargo:rustc-link-lib=static=hermesPublic");
        println!("cargo:rustc-link-lib=static=boost_context");
        println!("cargo:rustc-link-lib=framework=CoreFoundation");
    } else {
        // other platforms: plain shared library
        println!("cargo:rustc-link-lib=hermes");
        println!(
            "cargo:rustc-link-search={}",
            hermes_api_dir.to_string_lossy()
        );
    }
}
