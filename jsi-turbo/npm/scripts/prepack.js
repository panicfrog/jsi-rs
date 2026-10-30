#!/usr/bin/env node
/**
 * prepack: copy the jsi-rs Rust crates into the package so the published
 * tarball is self-sufficient (build.js locates them at <pkg>/rust/ and
 * the generated superlib's path dependencies resolve there).
 *
 * Runs on `npm pack` / `npm publish` only — local "file:" installs skip
 * it and keep using the jsi-rs checkout directly.
 *
 * The six crates are copied as siblings, which preserves their
 * `../<sibling>` path dependencies verbatim.
 */

const fs = require('fs');
const path = require('path');

// explicit --repo-root wins (build.js passes the checkout it located);
// default resolves the repo around a "file:"-installed package (node
// resolves the symlink, so __dirname is inside the real jsi-rs tree)
let repoRoot = null;
for (let i = 2; i < process.argv.length; i++) {
  if (process.argv[i] === '--repo-root') repoRoot = path.resolve(process.argv[++i]);
}
if (!repoRoot) repoRoot = path.resolve(__dirname, '..', '..', '..');

const crates = ['jsi', 'jsi-sys', 'jsi-macros', 'jsi-rn', 'jsi-turbo', 'jsi-turbo-macros'];
const dest = path.resolve(__dirname, '..', 'rust');

function copyDir(src, dst) {
  fs.mkdirSync(dst, {recursive: true});
  for (const e of fs.readdirSync(src, {withFileTypes: true})) {
    // build state, macro-generated bindings (rebuilt from source) and
    // the vendored submodule tree (jsi-sys/vendor symlinks to it; the
    // app's own React Native is used instead via JSI_DIR). 'npm' is this
    // very package — copying jsi-turbo would recurse into rust/ itself.
    if (e.name === 'target' || e.name === 'generated' || e.name === 'node_modules' || e.name === 'vendor' || e.name === 'npm') continue;
    const s = path.join(src, e.name);
    const d = path.join(dst, e.name);
    if (e.isDirectory()) copyDir(s, d);
    else fs.copyFileSync(s, d);
  }
}

fs.rmSync(dest, {recursive: true, force: true});
for (const crate of crates) {
  const src = path.join(repoRoot, crate);
  if (!fs.existsSync(path.join(src, 'Cargo.toml'))) {
    console.error(`prepack: ${src} has no Cargo.toml — run inside the jsi-rs repo`);
    process.exit(1);
  }
  copyDir(src, path.join(dest, crate));
  console.log(`prepack: ${crate} -> rust/${crate}`);
}

// fbjni headers for jsi-rn's Android build — not part of the
// react-native npm package, so they must ship with ours. Placed where
// jsi-rn's vendored fallback looks: rust/jsi-rn/vendor/fbjni/cxx
copyDir(path.join(repoRoot, 'vendor', 'fbjni', 'cxx'), path.join(dest, 'jsi-rn', 'vendor', 'fbjni', 'cxx'));
console.log('prepack: vendor/fbjni/cxx -> rust/jsi-rn/vendor/fbjni/cxx');
