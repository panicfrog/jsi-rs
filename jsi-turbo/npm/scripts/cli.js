#!/usr/bin/env node
/**
 * react-native-rust-turbo CLI.
 *
 *   npx react-native-rust-turbo init <package-name>
 *
 * Scaffolds a rust-turbo module package: npm metadata, the Rust crate and
 * a runnable demo module. Pure file generation — works from the registry
 * install, no jsi-rs checkout involved.
 */

const init = require('./init');

const [cmd, ...rest] = process.argv.slice(2);
if (cmd === 'init') {
  init(rest);
} else {
  console.error('usage: npx react-native-rust-turbo init <package-name>');
  process.exit(cmd ? 2 : 0);
}
