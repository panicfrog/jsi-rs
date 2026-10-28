/**
 * jsi-rs demo app
 *
 * The Rust module (native/jsi-demo-module) installs a `RustDemo` host object
 * into the JS global scope via JSI. Everything below calls into Rust.
 *
 * @format
 */

import React from 'react';
import {NativeModules, SafeAreaView, ScrollView, StatusBar, StyleSheet, Text, View} from 'react-native';

declare global {
  var RustDemo: {
    add(a: number, b: number): number;
    greet(name: string): string;
    sumList(nums: number[]): number;
    makePoint(x: number, y: number, label: string): {x: number; y: number; label: string};
    pointMagnitude(p: {x: number; y: number}): number;
    readonly version: string;
  };
}

// install the Rust module into the JSI runtime (synchronous, runs once)
NativeModules.JsiDemoModule?.install();

const results: Array<[string, string]> = (() => {
  const out: Array<[string, string]> = [];

  out.push(['RustDemo.version', RustDemo.version]);
  out.push(['RustDemo.add(19, 23)', String(RustDemo.add(19, 23))]);
  out.push(['RustDemo.greet("jsi-rs")', RustDemo.greet('jsi-rs')]);
  out.push(['RustDemo.sumList([1..100])', String(RustDemo.sumList(Array.from({length: 100}, (_, i) => i + 1)))]);

  const p = RustDemo.makePoint(3, 4, 'demo point');
  out.push(['RustDemo.makePoint(3, 4, ...)', JSON.stringify(p)]);
  out.push(['RustDemo.pointMagnitude(p)', String(RustDemo.pointMagnitude(p))]);

  return out;
})();

function App(): React.JSX.Element {
  const isDarkMode = false;

  const backgroundStyle = {
    backgroundColor: isDarkMode ? '#111' : '#fff',
  };

  return (
    <SafeAreaView style={backgroundStyle}>
      <StatusBar barStyle="dark-content" />
      <ScrollView contentInsetAdjustmentBehavior="automatic" style={backgroundStyle}>
        <View style={styles.container}>
          <Text style={styles.title}>jsi-rs demo</Text>
          <Text style={styles.subtitle}>
            Every value below was computed in Rust via JSI (React Native 0.77.3 + Hermes)
          </Text>
          {results.map(([label, value]) => (
            <View key={label} style={styles.row}>
              <Text style={styles.label}>{label}</Text>
              <Text style={styles.value}>{value}</Text>
            </View>
          ))}
        </View>
      </ScrollView>
    </SafeAreaView>
  );
}

const styles = StyleSheet.create({
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
