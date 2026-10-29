// A wrapper header b/c a bunch of JSI functions return opaque C++ types
// directly and autocxx currently isn't able to handle these headers b/c of
// stuff like nested protected structs
//
// NOTE: all jsi methods are called directly instead of through
// member-pointer casts. This keeps the file compatible with every jsi.h
// version from 0.73 to 0.88+: the convenience methods on Object/Function/
// Value etc. take `Runtime&` in older versions and `IRuntime&` in 0.88+,
// and a `Runtime&` argument converts to `IRuntime&` implicitly.

#ifndef JSI_WRAPPER_H
#define JSI_WRAPPER_H
#pragma once

#include <jsi/jsi.h>
#include <rust/cxx.h>

namespace jsi_rs
{
namespace ffi
{
using Buffer = ::facebook::jsi::Buffer;
using StringBuffer = ::facebook::jsi::StringBuffer;
using PreparedJavaScript = ::facebook::jsi::PreparedJavaScript;
using Symbol = ::facebook::jsi::Symbol;
using String = ::facebook::jsi::String;
using Object = ::facebook::jsi::Object;
using WeakObject = ::facebook::jsi::WeakObject;
using Array = ::facebook::jsi::Array;
using ArrayBuffer = ::facebook::jsi::ArrayBuffer;
using Function = ::facebook::jsi::Function;
using Value = ::facebook::jsi::Value;
using Instrumentation = ::facebook::jsi::Instrumentation;
using Scope = ::facebook::jsi::Scope;
using JSIException = ::facebook::jsi::JSIException;
using JSError = ::facebook::jsi::JSError;
using HostObject = ::facebook::jsi::HostObject;
using NativeState = ::facebook::jsi::NativeState;
using Runtime = ::facebook::jsi::Runtime;
using Pointer = ::facebook::jsi::Pointer;
using PropNameID = ::facebook::jsi::PropNameID;

using ConstPreparedJavaScript = const PreparedJavaScript;

// std::ffi::c_void is not supported by CXX
using c_void = void;

// HostObject

inline ::std::unique_ptr<Value> HostObject_get(
    ::facebook::jsi::HostObject &self, Runtime &rt, const PropNameID &name
) noexcept
{
  auto value = self.get(rt, name);
  return ::std::make_unique<Value>(std::move(value));
}

inline ::std::unique_ptr<std::vector<PropNameID>> HostObject_getPropertyNames(
    ::facebook::jsi::HostObject &self, Runtime &rt
) noexcept
{
  auto value = self.getPropertyNames(rt);
  return ::std::unique_ptr<std::vector<PropNameID>>(&value);
}

// Runtime

::std::shared_ptr<ConstPreparedJavaScript>
inline PreparedJavaScript_asConst(const ::std::shared_ptr<PreparedJavaScript> &js
) noexcept
{
  return ::std::shared_ptr<ConstPreparedJavaScript>(js);
}

inline ::std::unique_ptr<Value> Runtime_evaluateJavaScript(
    ::facebook::jsi::Runtime &self, const std::shared_ptr<Buffer> &buffer,
    rust::Str sourceURL
) noexcept
{
  auto value = self.evaluateJavaScript(buffer, std::string(sourceURL));
  return ::std::make_unique<Value>(std::move(value));
}

inline ::std::shared_ptr<ConstPreparedJavaScript> Runtime_prepareJavaScript(
    ::facebook::jsi::Runtime &self, const std::shared_ptr<Buffer> &buffer,
    rust::Str sourceURL
) noexcept
{
  return self.prepareJavaScript(buffer, std::string(sourceURL));
}

inline ::std::unique_ptr<Value> Runtime_evaluatePreparedJavaScript(
    ::facebook::jsi::Runtime &self,
    const ::std::shared_ptr<ConstPreparedJavaScript> &js
) noexcept
{
  auto value = self.evaluatePreparedJavaScript(js);
  return ::std::make_unique<Value>(std::move(value));
}

inline ::std::unique_ptr<Object> Runtime_global(::facebook::jsi::Runtime &self
) noexcept
{
  auto value = self.global();
  return ::std::make_unique<Object>(std::move(value));
}

::std::unique_ptr<std::string>
inline Runtime_description(::facebook::jsi::Runtime &self) noexcept
{
  auto value = self.description();
  return ::std::make_unique<std::string>(std::move(value));
}

// PropNameID

::std::unique_ptr<PropNameID>
inline PropNameID_forUtf8(Runtime &rt, rust::Str str) noexcept
{
  auto value =
      PropNameID::forUtf8(rt, (const uint8_t *)str.data(), str.length());
  return std::make_unique<PropNameID>(std::move(value));
}

::std::unique_ptr<PropNameID>
inline PropNameID_forString(Runtime &rt, const String &str) noexcept
{
  auto value = PropNameID::forString(rt, str);
  return std::make_unique<PropNameID>(std::move(value));
}

::std::unique_ptr<std::string>
inline PropNameID_toUtf8(const PropNameID &self, Runtime &rt) noexcept
{
  auto value = self.utf8(rt);
  return ::std::make_unique<std::string>(std::move(value));
}

inline bool PropNameID_compare(
    Runtime &rt, const PropNameID &self, const PropNameID &other
) noexcept
{
  return PropNameID::compare(rt, self, other);
}

inline std::unique_ptr<PropNameID> PropNameID_copy(const PropNameID &self, Runtime &rt)
{
  auto val = PropNameID(rt, self);
  return std::make_unique<PropNameID>(std::move(val));
}

// Symbol

::std::unique_ptr<std::string>
inline Symbol_toString(const Symbol &self, Runtime &rt) noexcept
{
  auto value = self.toString(rt);
  return ::std::make_unique<std::string>(std::move(value));
}

inline bool Symbol_compare(
    Runtime &rt, const Symbol &self, const Symbol &other
) noexcept
{
  return Symbol::strictEquals(rt, self, other);
}

// String

inline ::std::unique_ptr<String> String_fromUtf8(Runtime &rt, rust::Str str) noexcept
{
  auto value =
      String::createFromUtf8(rt, (const uint8_t *)str.data(), str.length());
  return std::make_unique<String>(std::move(value));
}

::std::unique_ptr<std::string>
inline String_toString(const String &self, Runtime &rt) noexcept
{
  auto value = self.utf8(rt);
  return ::std::make_unique<std::string>(std::move(value));
}

inline bool String_compare(
    Runtime &rt, const String &self, const String &other
) noexcept
{
  return String::strictEquals(rt, self, other);
}

// Object

inline ::std::unique_ptr<Object> Object_create(Runtime &rt) noexcept
{
  return std::make_unique<Object>(rt);
}

inline ::std::unique_ptr<Object> Object_createFromHostObjectShared(
    Runtime &rt, std::shared_ptr<HostObject> ho
) noexcept
{
  auto value = facebook::jsi::Object::createFromHostObject(rt, ho);
  return std::make_unique<Object>(std::move(value));
}

inline ::std::unique_ptr<Object> Object_createFromHostObjectUnique(
    Runtime &rt, std::unique_ptr<HostObject> ho
) noexcept
{
  std::shared_ptr<HostObject> s_ho = std::move(ho);
  auto value = facebook::jsi::Object::createFromHostObject(rt, s_ho);
  return std::make_unique<Object>(std::move(value));
}

inline bool Object_compare(
    Runtime &rt, const Object &self, const Object &other
) noexcept
{
  return Object::strictEquals(rt, self, other);
}

inline std::unique_ptr<Value> Object_getProperty(
    const Object &self, Runtime &rt, const PropNameID &name
) noexcept
{
  auto value = self.getProperty(rt, name);
  return std::make_unique<Value>(std::move(value));
}

inline void Object_setProperty(
    Object &self, Runtime &rt, const PropNameID &name, Value const &value
) noexcept
{
  self.setProperty(rt, name, value);
}

inline std::unique_ptr<Array> Object_asArray(Object const &self, Runtime &rt)
{
  auto val = self.asArray(rt);
  return std::make_unique<Array>(std::move(val));
}

std::unique_ptr<ArrayBuffer>
inline Object_asArrayBuffer(Object const &self, Runtime &rt)
{
  auto val = self.getArrayBuffer(rt);
  return std::make_unique<ArrayBuffer>(std::move(val));
}

inline std::unique_ptr<Function> Object_asFunction(Object const &self, Runtime &rt)
{
  auto val = self.asFunction(rt);
  return std::make_unique<Function>(std::move(val));
}

inline std::shared_ptr<HostObject> Object_asHostObject(Object const &self, Runtime &rt)
{
  return self.asHostObject(rt);
}

inline bool Object_instanceOf(Object const &self, Runtime &rt, Function const &ctor)
{
  return self.instanceOf(rt, ctor);
}

inline bool Object_hasProperty(Object const &self, Runtime &rt, const PropNameID &name)
{
  return self.hasProperty(rt, name);
}

inline bool Object_isArray(Object const &self, Runtime &rt)
{
  return self.isArray(rt);
}

// NativeState

inline bool Runtime_hasNativeState(
    Runtime &rt, Object const &obj
) noexcept
{
  return rt.hasNativeState(obj);
}

inline ::std::shared_ptr<NativeState> Runtime_getNativeState(
    Runtime &rt, Object const &obj
) noexcept
{
  // NOTE: the caller must check hasNativeState first; Hermes asserts if the
  // object has no native state
  return rt.getNativeState(obj);
}

inline void Object_setNativeState(
    Object const &self, Runtime &rt, ::std::shared_ptr<NativeState> state
) noexcept
{
  self.setNativeState(rt, std::move(state));
}

inline ::std::unique_ptr<Object> Object_createWithPrototype(
    Runtime &rt, Value const &prototype
) noexcept
{
  auto value = Object::create(rt, prototype);
  return std::make_unique<Object>(std::move(value));
}

inline void Object_setExternalMemoryPressure(
    Object const &self, Runtime &rt, size_t amt
) noexcept
{
  self.setExternalMemoryPressure(rt, amt);
}

inline bool Object_isArrayBuffer(Object const &self, Runtime &rt)
{
  return self.isArrayBuffer(rt);
}

inline bool Object_isFunction(Object const &self, Runtime &rt)
{
  return self.isFunction(rt);
}

inline std::unique_ptr<Array> Object_getPropertyNames(Object &self, Runtime &rt)
{
  auto val = self.getPropertyNames(rt);
  return std::make_unique<Array>(std::move(val));
}

// WeakObject

std::unique_ptr<WeakObject>
inline WeakObject_fromObject(Runtime &rt, Object const &object)
{
  return std::make_unique<WeakObject>(rt, object);
}

inline std::unique_ptr<Value> WeakObject_lock(WeakObject &self, Runtime &rt)
{
  auto val = self.lock(rt);
  return std::make_unique<Value>(std::move(val));
}

// Array

inline std::unique_ptr<Array> Array_createWithLength(Runtime &rt, size_t length)
{
  return std::make_unique<Array>(rt, length);
}

inline std::unique_ptr<Value> Array_get(Array const &self, Runtime &rt, size_t index)
{
  auto val = self.getValueAtIndex(rt, index);
  return std::make_unique<Value>(std::move(val));
}

inline void Array_set(Array &self, Runtime &rt, size_t index, Value const &value)
{
  self.setValueAtIndex(rt, index, value);
}

inline size_t Array_length(Array const &self, Runtime &rt)
{
  return self.length(rt);
}

inline size_t ArrayBuffer_length(ArrayBuffer const &self, Runtime &rt)
{
  return self.length(rt);
}

inline uint8_t *ArrayBuffer_data(ArrayBuffer const &self, Runtime &rt)
{
  return self.data(rt);
}

// A MutableBuffer backed by Rust-owned memory. `ctx` is a boxed Rust
// allocation; `drop` frees it. The deleter runs when the JS GC releases the
// ArrayBuffer, so ownership effectively moves from Rust to the runtime.
class RustMutableBuffer final : public ::facebook::jsi::MutableBuffer
{
public:
  RustMutableBuffer(uint8_t *data, size_t size, void *ctx,
                    ::rust::Fn<void(void *)> drop)
      : data_(data), size_(size), ctx_(ctx), drop_(drop) {}
  RustMutableBuffer(const RustMutableBuffer &) = delete;
  RustMutableBuffer &operator=(const RustMutableBuffer &) = delete;
  ~RustMutableBuffer() override { drop_(ctx_); }

  size_t size() const override { return size_; }
  uint8_t *data() override { return data_; }

private:
  uint8_t *data_;
  size_t size_;
  void *ctx_;
  ::rust::Fn<void(void *)> drop_;
};

inline std::unique_ptr<ArrayBuffer> Runtime_createArrayBufferFromRust(
    Runtime &rt, uint8_t *data, size_t size, void *ctx,
    ::rust::Fn<void(void *)> drop
)
{
  auto buffer = std::make_shared<RustMutableBuffer>(data, size, ctx, drop);
  auto value = rt.createArrayBuffer(std::move(buffer));
  return std::make_unique<ArrayBuffer>(std::move(value));
}

inline std::unique_ptr<Value> Value_fromArrayBuffer(
    Runtime &rt, std::unique_ptr<ArrayBuffer> b
)
{
  return std::make_unique<Value>(rt, std::move(*b.release()));
}

inline bool Function_isHostFunction(Function const &self, Runtime &rt)
{
  return self.isHostFunction(rt);
}

// Function

std::unique_ptr<Value>
inline Function_call(Function const &self, Runtime &rt, std::vector<Value> const &args)
{
  auto value = self.call(rt, args.data(), args.size());
  return std::make_unique<Value>(std::move(value));
}

inline std::unique_ptr<Value> Function_callWithThis(
    Function const &self, Runtime &rt, Object const &thisObj,
    std::vector<Value> const &args
)
{
  auto value = self.callWithThis(rt, thisObj, args.data(), args.size());
  return std::make_unique<Value>(std::move(value));
}

inline std::unique_ptr<Value> Function_callAsConstructor(
    Function const &self, Runtime &rt, std::vector<Value> const &args
)
{
  auto value = self.callAsConstructor(rt, args.data(), args.size());
  return std::make_unique<Value>(std::move(value));
}

::std::unique_ptr<::facebook::jsi::Value> host_fn_trampoline(
    ::facebook::jsi::Runtime &rt, const ::facebook::jsi::Value &thisVal,
    const ::facebook::jsi::Value *args, ::std::uint32_t count,
    ::std::size_t stride, ::jsi_rs::ffi::c_void *closure
);

inline std::unique_ptr<Function> Function_createFromHostFunction(
    Runtime &rt, const PropNameID &name, unsigned int paramCount, void *closure
)
{
  auto value = Function::createFromHostFunction(
      rt, name, paramCount,
      [closure](
          Runtime &rt, const Value &thisVal, const Value *args, size_t count
      ) {
        auto stride = sizeof(Value);
        auto val =
            host_fn_trampoline(rt, thisVal, args, count, stride, closure);
        return std::move(*val.release());
      }
  );
  return std::make_unique<Function>(std::move(value));
}

// Value

inline std::unique_ptr<Value> Value_fromUndefined()
{
  return std::make_unique<Value>();
}

inline std::unique_ptr<Value> Value_fromNull()
{
  return std::make_unique<Value>(std::nullptr_t());
}

inline std::unique_ptr<Value> Value_fromInt(int i)
{
  return std::make_unique<Value>(i);
}

inline std::unique_ptr<Value> Value_fromBool(bool b)
{
  return std::make_unique<Value>(b);
}

inline std::unique_ptr<Value> Value_fromDouble(double d)
{
  return std::make_unique<Value>(d);
}

inline std::unique_ptr<Value> Value_fromString(Runtime &rt, std::unique_ptr<String> s)
{
  return std::make_unique<Value>(rt, std::move(*s.release()));
}

inline std::unique_ptr<Value> Value_fromSymbol(Runtime &rt, std::unique_ptr<Symbol> s)
{
  return std::make_unique<Value>(rt, std::move(*s.release()));
}

inline std::unique_ptr<Value> Value_fromObject(Runtime &rt, std::unique_ptr<Object> o)
{
  return std::make_unique<Value>(rt, std::move(*o.release()));
}

inline std::unique_ptr<Value> Value_copyFromString(Runtime &rt, const String &s)
{
  return std::make_unique<Value>(rt, s);
}

inline std::unique_ptr<Value> Value_copyFromSymbol(Runtime &rt, const Symbol &s)
{
  return std::make_unique<Value>(rt, s);
}

inline std::unique_ptr<Value> Value_copyFromObject(Runtime &rt, const Object &o)
{
  return std::make_unique<Value>(rt, o);
}

inline std::unique_ptr<Value> Value_fromJson(Runtime &rt, rust::Str s)
{
  auto val =
      Value::createFromJsonUtf8(rt, (const uint8_t *)s.data(), s.length());
  return std::make_unique<Value>(std::move(val));
}

inline bool Value_compare(Runtime &rt, const Value &self, const Value &other) noexcept
{
  return Value::strictEquals(rt, self, other);
}

inline std::unique_ptr<String> Value_asString(const Value &self, Runtime &rt)
{
  auto val = self.asString(rt);
  return std::make_unique<String>(std::move(val));
}

inline std::unique_ptr<Symbol> Value_asSymbol(const Value &self, Runtime &rt)
{
  auto val = self.asSymbol(rt);
  return std::make_unique<Symbol>(std::move(val));
}

inline std::unique_ptr<Object> Value_asObject(const Value &self, Runtime &rt)
{
  auto val = self.asObject(rt);
  return std::make_unique<Object>(std::move(val));
}

inline std::unique_ptr<String> Value_toString(const Value &self, Runtime &rt)
{
  auto val = self.toString(rt);
  return std::make_unique<String>(std::move(val));
}

inline std::unique_ptr<Value> Value_copy(const Value &self, Runtime &rt)
{
  auto val = Value(rt, self);
  return std::make_unique<Value>(std::move(val));
}

// Utility fns

inline std::unique_ptr<std::vector<Value>> create_value_vector()
{
  return std::make_unique<std::vector<Value>>();
}

inline void push_value_vector(std::vector<Value> &values, std::unique_ptr<Value> value)
{
  auto x = std::move(*value.release());
  values.push_back(std::move(x));
}

inline std::unique_ptr<std::vector<PropNameID>> create_prop_name_vector()
{
  return std::make_unique<std::vector<PropNameID>>();
}

inline void push_prop_name_vector(
    std::vector<PropNameID> &values, std::unique_ptr<PropNameID> value
)
{
  auto x = std::move(*value.release());
  values.push_back(std::move(x));
}

inline std::unique_ptr<PropNameID> pop_prop_name_vector(std::vector<PropNameID> &values
)
{
  if (values.size() > 0) {
    auto prop = std::move(values.back());
    values.pop_back();
    return std::make_unique<PropNameID>(std::move(prop));
  } else {
    return std::unique_ptr<PropNameID>(nullptr);
  }
}
} // namespace ffi
} // namespace jsi_rs

#endif
