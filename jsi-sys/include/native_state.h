// A jsi::NativeState that owns a Rust object, used to attach Rust module
// instances to plain JS objects (the Nitro-style pattern: methods live on a
// prototype, `this` resolves through the object's NativeState).
//
// Dropping the JS object's native state (GC) drops the Rust object — the
// rust::Box destructor runs on the C++ side.

#ifndef JSI_NATIVE_STATE_H
#define JSI_NATIVE_STATE_H
#pragma once

#include "rust/cxx.h"
#include "jsi/jsi.h"

namespace jsi_rs
{
  namespace ffi
  {
    using NativeState = ::facebook::jsi::NativeState;

    // Rust-side type declared in the cxx bridge (ffi/native_state.rs); holds
    // an opaque Box<dyn Any + Send + Sync> — the Rust module instance.
    struct RustNativeCarrier;

    class RustNativeState : public NativeState
    {
    public:
      rust::Box<RustNativeCarrier> state;

      RustNativeState(rust::Box<RustNativeCarrier> state) : NativeState(), state(std::move(state)) {}
    };

    inline ::std::shared_ptr<NativeState> RustNativeState_create(
        rust::Box<RustNativeCarrier> state) noexcept
    {
      return std::make_shared<RustNativeState>(std::move(state));
    }

    // Returns a null shared_ptr if the given state is not a RustNativeState
    // (e.g. some other framework's state object).
    inline ::std::shared_ptr<RustNativeState> NativeState_asRust(
        ::std::shared_ptr<NativeState> state) noexcept
    {
      return std::dynamic_pointer_cast<RustNativeState>(state);
    }

    inline RustNativeCarrier const &RustNativeState_getInner(
        RustNativeState const &state) noexcept
    {
      return *state.state;
    }
  }
}

#endif
