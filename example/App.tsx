/**
 * jsi-rs demo app
 *
 * The Rust module (native/jsi-demo-module) installs a `RustDemo` host object
 * into the JS global scope via JSI. Everything below calls into Rust.
 *
 * @format
 */

import React from 'react';
import {
  SafeAreaView,
  ScrollView,
  StatusBar,
  StyleSheet,
  Text,
  TurboModuleRegistry,
  View,
} from 'react-native';

declare global {
  var RustDemo:
    | {
        add(a: number, b: number): number;
        greet(name: string): string;
        sumList(nums: number[]): number;
        makePoint(x: number, y: number, label: string): {x: number; y: number; label: string};
        pointMagnitude(p: {x: number; y: number}): number;
        readonly version: string;
      }
    | undefined;
}

// Requesting the TurboModule makes React Native hand the JS runtime to the
// native side (iOS: installJSIBindingsWithRuntime:, Android:
// getBindingsInstaller()), which installs the `RustDemo` global.
TurboModuleRegistry.get('JsiDemoModule');

function computeResults(rust: NonNullable<typeof RustDemo>): Array<[string, string]> {
  const out: Array<[string, string]> = [];

  out.push(['RustDemo.version', rust.version]);
  out.push(['RustDemo.add(19, 23)', String(rust.add(19, 23))]);
  out.push(['RustDemo.greet("jsi-rs")', rust.greet('jsi-rs')]);
  out.push(['RustDemo.sumList([1..100])', String(rust.sumList(Array.from({length: 100}, (_, i) => i + 1)))]);

  const p = rust.makePoint(3, 4, 'demo point');
  out.push(['RustDemo.makePoint(3, 4, ...)', JSON.stringify(p)]);
  out.push(['RustDemo.pointMagnitude(p)', String(rust.pointMagnitude(p))]);

  return out;
}

function App(): React.JSX.Element {
  const rust = globalThis.RustDemo;

  return (
    <SafeAreaView style={styles.background}>
      <StatusBar barStyle="dark-content" />
      <ScrollView contentInsetAdjustmentBehavior="automatic" style={styles.background}>
        <View style={styles.container}>
          <Text style={styles.title}>jsi-rs demo</Text>
          {rust === undefined ? (
            <Text style={styles.error}>
              The Rust module was not installed: the `RustDemo` global does not exist.
            </Text>
          ) : (
            <>
              <Text style={styles.subtitle}>
                Every value below was computed in Rust via JSI (React Native 0.88.1 + Hermes)
              </Text>
              {computeResults(rust).map(([label, value]) => (
                <View key={label} style={styles.row}>
                  <Text style={styles.label}>{label}</Text>
                  <Text style={styles.value}>{value}</Text>
                </View>
              ))}
            </>
          )}
        </View>
      </ScrollView>
    </SafeAreaView>
  );
}

const styles = StyleSheet.create({
  background: {
    backgroundColor: '#fff',
  },
  error: {
    fontSize: 16,
    color: '#c00',
    marginTop: 12,
  },
  container: {
    padding: 24,
    gap: 8,
  },
  title: {
    fontSize: 28,
    fontWeight: '700',
    marginBottom: 4,
    color: '#000',
  },
  subtitle: {
    fontSize: 14,
    color: '#666',
    marginBottom: 20,
  },
  row: {
    flexDirection: 'row',
    justifyContent: 'space-between',
    paddingVertical: 10,
    borderBottomWidth: StyleSheet.hairlineWidth,
    borderBottomColor: '#ddd',
    gap: 12,
  },
  label: {
    fontSize: 14,
    color: '#333',
    flexShrink: 1,
  },
  value: {
    fontSize: 14,
    fontWeight: '600',
    color: '#0a7',
    flexShrink: 1,
    textAlign: 'right',
  },
});

export default App;
