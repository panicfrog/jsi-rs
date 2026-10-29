#ifndef JSI_RN_WRAPPER_H
#define JSI_RN_WRAPPER_H
#pragma once

#include <ReactCommon/CallInvoker.h>
#include <rust/cxx.h>

namespace jsi_rs
{
namespace rn
{
// std::ffi::c_void is not supported by CXX
using c_void = void;

inline void call_invoker_trampoline(c_void *closure);

inline void CallInvoker_invokeSync(
    std::shared_ptr<facebook::react::CallInvoker> ci, c_void *closure
)
{
  ci->invokeSync([closure]() { call_invoker_trampoline(closure); });
}

inline void CallInvoker_invokeAsync(
    std::shared_ptr<facebook::react::CallInvoker> ci, c_void *closure
)
{
  ci->invokeAsync([closure]() { call_invoker_trampoline(closure); });
}
} // namespace rn
} // namespace jsi_rs

#endif
