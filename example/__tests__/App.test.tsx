/**
 * @format
 */

import React from 'react';
import ReactTestRenderer from 'react-test-renderer';
import App from '../App';

// The generated TypeScript bindings bootstrap the native RustTurbo proxy on
// first use — in the jest environment there is no runtime, so serve the
// modules as inert stubs.
jest.mock('jsi-demo-module/generated/rust-turbo/TurboDemo', () => ({
  getTurboDemo: () =>
    ({
      add: () => 0,
      greet: () => '',
      sumList: () => 0,
      makePoint: () => ({x: 0, y: 0, label: ''}),
      pointMagnitude: () => 0,
      slowAdd: async () => 0,
      makeBuffer: () => new ArrayBuffer(0),
      checksum: () => 0,
      createCounter: () => ({increment: () => undefined, value: 0}),
      readCounter: () => 0,
      version: 'test',
    }) as never,
}));

jest.mock('jsi-demo-module/generated/rust-turbo/Ticker', () => ({
  getTicker: () =>
    ({
      onTick: () => undefined,
      fireTicks: () => 0,
      fireTicksAsync: async () => 0,
    }) as never,
}));

test('renders correctly', async () => {
  await ReactTestRenderer.act(() => {
    ReactTestRenderer.create(<App />);
  });
});
