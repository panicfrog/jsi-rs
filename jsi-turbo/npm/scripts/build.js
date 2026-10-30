#!/usr/bin/env node
/**
 * react-native-rust-turbo build hook.
 *
 * Discovers every installed npm package that carries a `rust-turbo` field,
 * generates ONE aggregate cargo crate (superlib) depending on all their
 * Rust crates, and builds it into a single static library for the given
 * target. Platform build hooks (pod script_phase / gradle task) call this
 * script; humans can run it directly too.
 *
 * Module packages declare themselves in package.json:
 *
 *   "rust-turbo": {
 *     "crate": "rust",              // crate dir inside the package
 *     "modules": ["TurboDemo"]      // #[RustTurboModule] names (for the
 *   }                               //  iOS FORCE_LINK anchors)
 *
 * Usage:
 *   node build.js --app <app-root> --target aarch64-apple-ios-sim [--release]
 *   node build.js --app <app-root> --target aarch64-linux-android --release --ndk <ndk-dir>
 *
 * Outputs <core-pkg>/generated/rust-app/target/<target>/release/librust_app.a
 * and prints its path on the last line.
 */

const fs = require('fs');
const path = require('path');
const {execFileSync} = require('child_process');

function parseArgs(argv) {
  const args = {release: false};
  for (let i = 2; i < argv.length; i++) {
    const a = argv[i];
    if (a === '--app') args.app = argv[++i];
    else if (a === '--target') args.target = argv[++i];
    else if (a === '--release') args.release = true;
    else if (a === '--ndk') args.ndk = argv[++i];
    else if (a === '--android-api') args.androidApi = argv[++i];
    else if (a === '--jsi-rs') args.jsiRs = argv[++i];
    else {
      console.error(`unknown argument: ${a}`);
      process.exit(2);
    }
  }
  if (!args.app || !args.target) {
    console.error('usage: build.js --app <app-root> --target <rust-triple> [--release] [--ndk <dir>] [--jsi-rs <dir>]');
    process.exit(2);
  }
  return args;
}

/** Rust module packages installed in <app>/node_modules. */
function discover(appRoot) {
  const nm = path.join(appRoot, 'node_modules');
  const found = [];
  let entries = [];
  try {
    entries = fs.readdirSync(nm, {withFileTypes: true});
  } catch {
    console.error(`no node_modules at ${nm}; run npm install first`);
    process.exit(1);
  }
  // scoped packages (@scope/name) live one level deeper; local "file:"
  // installs are symlinks, which readdir dirents report as links, not dirs
  const isDir = p => {
    try {
      return fs.statSync(p).isDirectory();
    } catch {
      return false;
    }
  };
  const dirs = [];
  for (const e of entries) {
    if (e.name.startsWith('.')) continue;
    const full = path.join(nm, e.name);
    if (!isDir(full)) continue;
    if (e.name.startsWith('@')) {
      for (const sub of fs.readdirSync(full, {withFileTypes: true})) {
        if (isDir(path.join(full, sub.name))) dirs.push(path.join(e.name, sub.name));
      }
    } else {
      dirs.push(e.name);
    }
  }
  for (const dir of dirs) {
    const pj = path.join(nm, dir, 'package.json');
    if (!fs.existsSync(pj)) continue;
    const meta = JSON.parse(fs.readFileSync(pj, 'utf8'));
    const rt = meta['rust-turbo'];
    if (!rt) continue;
    if (!rt.crate || !Array.isArray(rt.modules) || rt.modules.length === 0) {
      console.error(`${meta.name}: "rust-turbo" needs "crate" and non-empty "modules"`);
      process.exit(1);
    }
    found.push({
      name: meta.name,
      dir: path.join(nm, dir),
      crateDir: path.join(nm, dir, rt.crate),
      // the crate's real package name (used verbatim in the dependency
      // declaration; cargo matches path deps by exact name)
      crateName: rt.crateName || path.basename(rt.crate),
      modules: rt.modules,
    });
  }
  return found;
}

/** TurboDemo -> turbo_demo (matches heck ToSnakeCase in the macro). */
function snake(name) {
  return name
    .replace(/([a-z0-9])([A-Z])/g, '$1_$2')
    .replace(/([A-Z]+)([A-Z][a-z])/g, '$1_$2')
    .toLowerCase();
}

function generateSuperlib(appRoot, packages, jsiRsDir) {
  // Keep generated build state OUT of the npm package directory: local
  // "file:" installs are symlinks, and the RN autolinking CLI walks
  // dependency trees — a cargo target/ dir inside the package makes it OOM.
  // node_modules/.cache is the conventional, hidden-from-scanners location.
  const genDir = path.join(appRoot, 'node_modules', '.cache', 'rust-turbo', 'rust-app');
  fs.mkdirSync(path.join(genDir, 'src'), {recursive: true});
  fs.mkdirSync(path.join(genDir, '.cargo'), {recursive: true});

  const rel = p => path.relative(genDir, p);

  const deps = packages
    .map(p => `${JSON.stringify(p.crateName).replace(/"/g, '')} = { path = ${JSON.stringify(rel(p.crateDir))} }`)
    .join('\n');
  // Rust identifier form of the crate name for code references
  const ident = name => name.replace(/-/g, '_');
  const anchors = packages
    .flatMap(p => p.modules.map(m => `    ${ident(p.crateName)}::__jsi_turbo_force_${snake(m)},`))
    .join('\n');

  fs.writeFileSync(
    path.join(genDir, 'Cargo.toml'),
    `# GENERATED by react-native-rust-turbo — do not edit.
[package]
name = "rust-app"
version = "0.1.0"
edition = "2021"
publish = false

[lib]
crate-type = ["staticlib"]

[dependencies]
jsi = { path = ${JSON.stringify(rel(path.join(jsiRsDir, 'jsi')))}, default-features = false, features = ["macros", "serde"] }
jsi-sys = { path = ${JSON.stringify(rel(path.join(jsiRsDir, 'jsi-sys')))} }
jsi-rn = { path = ${JSON.stringify(rel(path.join(jsiRsDir, 'jsi-rn')))} }
jsi-turbo = { path = ${JSON.stringify(rel(path.join(jsiRsDir, 'jsi-turbo')))} }
anyhow = "1.0"
cxx = "1.0"

${deps}

[build-dependencies]
cxx-build = "1.0"

[workspace]
`
  );

  fs.writeFileSync(
    path.join(genDir, 'build.rs'),
    `fn main() {
    cxx_build::bridges(vec!["src/lib.rs"]).flag_if_supported("-std=c++17").compile("rust-app");
}
`
  );

  fs.writeFileSync(
    path.join(genDir, 'src', 'lib.rs'),
    `//! GENERATED by react-native-rust-turbo — the aggregate superlib for this
//! app: depends on every installed rust-turbo module package.

use std::pin::Pin;

#[cxx::bridge]
mod bridge {
    unsafe extern "C++" {
        include!("wrapper.h");
        include!("rn_wrapper.h");
        #[namespace = "facebook::jsi"]
        type Runtime = jsi_sys::Runtime;
        #[namespace = "facebook::react"]
        type CallInvoker = jsi_rn::sys::CallInvoker;
    }
    extern "Rust" {
        fn rust_turbo_install(rt: Pin<&mut Runtime>, invoker: SharedPtr<CallInvoker>) -> bool;
    }
}

// keeps every module crate's objects (and their linkme entries) alive
// through iOS staticlib dead-code elimination
#[used]
static FORCE_LINK: [extern "C" fn(); ${packages.reduce((n, p) => n + p.modules.length, 0)}] = [
${anchors}
];

fn rust_turbo_install(rt: Pin<&mut jsi_sys::Runtime>, invoker: cxx::SharedPtr<jsi_rn::sys::CallInvoker>) -> bool {
    let rt = unsafe { Pin::into_inner_unchecked(rt) };
    let raw = rt as *mut jsi_sys::Runtime;
    let (mut rt, invoker) = jsi_rn::init(raw, invoker);
    let dispatcher = jsi_turbo::Dispatcher::call_invoker(invoker, raw);
    match jsi_turbo::install(&mut rt, dispatcher) {
        Ok(()) => true,
        Err(e) => {
            eprintln!("rust-app: failed to install RustTurbo: {e:#}");
            false
        }
    }
}
`
  );

  fs.writeFileSync(
    path.join(genDir, '.cargo', 'config.toml'),
    `# GENERATED by react-native-rust-turbo — point jsi-sys/jsi-rn at the app's
# own React Native (relative to this crate).
[env]
JSI_DIR = { value = ${JSON.stringify(rel(path.join(appRoot, 'node_modules/react-native/ReactCommon/jsi')))}, relative = true }
REACT_NATIVE_DIR = { value = ${JSON.stringify(rel(path.join(appRoot, 'node_modules/react-native')))}, relative = true }
`
  );

  return genDir;
}

function locateJsiRs(args, appRoot) {
  if (args.jsiRs) return path.resolve(args.jsiRs);
  // 1. the app can pin it: "rust-turbo": { "jsi-rs": "<path>" } in its own package.json
  const appPj = JSON.parse(fs.readFileSync(path.join(appRoot, 'package.json'), 'utf8'));
  const fromApp = appPj['rust-turbo'] && appPj['rust-turbo']['jsi-rs'];
  if (fromApp) return path.resolve(appRoot, fromApp);
  // 2. env var
  if (process.env.JSI_RS_DIR) return path.resolve(process.env.JSI_RS_DIR);
  // 3. this checkout itself (works when developing inside the jsi-rs repo)
  const local = path.resolve(__dirname, '..', '..', '..');
  if (fs.existsSync(path.join(local, 'jsi-turbo', 'Cargo.toml'))) return local;
  // 4. the published package ships the crates itself (prepack copies them
  //    to <pkg>/rust/ as siblings, so path deps keep resolving)
  const shipped = path.resolve(__dirname, '..', 'rust');
  if (fs.existsSync(path.join(shipped, 'jsi', 'Cargo.toml'))) return null;
  console.error(
    'jsi-rs checkout not found. Set JSI_RS_DIR or add\n' +
      '  "rust-turbo": { "jsi-rs": "<relative path to the jsi-rs repo>" }\n' +
      'to the app\'s package.json.'
  );
  process.exit(1);
}

/**
 * The crates the superlib (and module crates) depend on always come from
 * this package's own rust/ directory — ONE copy in the cargo graph (a
 * second copy fails on jsi-sys's `links = "jsi"`). In a registry install
 * that dir ships with the tarball; in a "file:" dev install it doesn't
 * exist, so we sync it from the located checkout on every build (cheap:
 * pure source, ~500 files).
 */
function ensureShippedCrates(checkout, appRoot) {
  // Always referenced through the app's node_modules — the same path
  // string module packages use. cargo does NOT canonicalize paths, so
  // mixing e.g. <repo>/jsi-turbo/npm/rust/jsi (the real path behind a
  // "file:" symlink) with node_modules/react-native-rust-turbo/rust/jsi
  // would put TWO jsi-sys copies in the graph (links = "jsi" conflict).
  const shipped = path.join(appRoot, 'node_modules', 'react-native-rust-turbo', 'rust');
  if (checkout) {
    // dev ("file:") install: rust/ doesn't ship, sync it from the checkout
    execFileSync('node', [path.resolve(__dirname, 'prepack.js'), '--repo-root', checkout], {stdio: 'inherit'});
  }
  return shipped;
}

/**
 * Version-consistency warning: jsi.h is the ABI contract between the Rust
 * side and the app's engine, so a major.minor drift between the app's
 * react-native and this jsi-rs checkout is worth knowing about (patch
 * releases are fine). Warn, don't fail — newer RN versions often still work.
 */
function checkVersions(appRoot, jsiRsDir) {
  const jsiToml = fs.readFileSync(path.join(jsiRsDir, 'jsi', 'Cargo.toml'), 'utf8');
  const m = jsiToml.match(/^version\s*=\s*"([^"]+)"/m);
  if (!m) return; // can't tell; the cargo build will surface real problems
  const jsiRsVersion = m[1];
  const rnPjPath = path.join(appRoot, 'node_modules', 'react-native', 'package.json');
  let rnVersion;
  try {
    rnVersion = JSON.parse(fs.readFileSync(rnPjPath, 'utf8')).version;
  } catch {
    console.error(
      `rust-turbo: WARNING no react-native found at ${rnPjPath} — jsi.h comes from there; the cargo build will fail unless JSI_DIR is set.`
    );
    return;
  }
  const short = v => v.split('.').slice(0, 2).join('.');
  if (short(jsiRsVersion) !== short(rnVersion)) {
    console.error(
      `rust-turbo: WARNING app uses react-native ${rnVersion} but this jsi-rs targets ${jsiRsVersion}. ` +
        `jsi.h may have changed; if the build or runtime misbehaves, use the jsi-rs branch for ${short(rnVersion)}.`
    );
  }
}

function ndkToolchainEnv(ndkDir, target, apiLevel) {
  const prebuilt = path.join(ndkDir, 'toolchains', 'llvm', 'prebuilt');
  let entries = [];
  try {
    entries = fs.readdirSync(prebuilt);
  } catch {
    console.error(`ndk toolchains dir not found at ${prebuilt} (wrong --ndk path?)`);
    process.exit(1);
  }
  const hostTag = entries.find(e => fs.existsSync(path.join(prebuilt, e, 'bin')));
  if (!hostTag) {
    console.error(`no toolchain host dir under ${prebuilt}`);
    process.exit(1);
  }
  const bin = path.join(prebuilt, hostTag, 'bin');
  const api = apiLevel || 24;
  const clang = `${target}${api}-clang`;
  const env = {};
  env[`CARGO_TARGET_${target.toUpperCase().replace(/-/g, '_')}_LINKER`] = path.join(bin, clang);
  const envKey = target.replace(/-/g, '_');
  env[`CC_${envKey}`] = path.join(bin, clang);
  env[`CXX_${envKey}`] = path.join(bin, `${clang}++`);
  env[`AR_${envKey}`] = path.join(bin, 'llvm-ar');
  return env;
}

function main() {
  const args = parseArgs(process.argv);
  const appRoot = path.resolve(args.app);
  const checkout = locateJsiRs(args, appRoot); // null when the package ships its own crates
  const crateSource = ensureShippedCrates(checkout, appRoot);
  checkVersions(appRoot, crateSource);

  const packages = discover(appRoot);
  if (packages.length === 0) {
    console.error('no rust-turbo module packages found in node_modules');
    process.exit(1);
  }
  console.log(
    'rust-turbo packages:\n' +
      packages.map(p => `  ${p.name} (crate: ${p.crateName}, modules: ${p.modules.join(', ')})`).join('\n')
  );

  const genDir = generateSuperlib(appRoot, packages, crateSource);

  const profile = args.release ? 'release' : 'debug';
  const cargoArgs = ['build', '--manifest-path', path.join(genDir, 'Cargo.toml'), '--target', args.target];
  if (args.release) cargoArgs.push('--release');
  console.log(`cargo ${cargoArgs.join(' ')}`);
  const env = {...process.env};
  // jsi.h comes from the app's own React Native. Injected as real env
  // vars because cargo only reads .cargo/config.toml from the WORKING
  // directory upward — the generated one next to the manifest (kept for
  // humans running cargo manually in genDir) is invisible to this call.
  env.JSI_DIR = path.join(appRoot, 'node_modules', 'react-native', 'ReactCommon', 'jsi');
  env.REACT_NATIVE_DIR = path.join(appRoot, 'node_modules', 'react-native');
  if (args.target.includes('android')) {
    const ndk = args.ndk || process.env.ANDROID_NDK_HOME || process.env.NDK_HOME;
    if (!ndk) {
      console.error('Android builds need --ndk <dir> (or ANDROID_NDK_HOME)');
      process.exit(1);
    }
    Object.assign(env, ndkToolchainEnv(path.resolve(ndk), args.target, args.androidApi));
  }
  execFileSync('cargo', cargoArgs, {stdio: 'inherit', env});

  const lib = path.join(genDir, 'target', args.target, profile, 'librust_app.a');
  if (!fs.existsSync(lib)) {
    console.error(`expected ${lib} after a successful build`);
    process.exit(1);
  }
  console.log(lib);
}

main();
