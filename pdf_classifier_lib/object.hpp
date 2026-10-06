#pragma once

#include "capability.hpp"
#include "result.hpp"
#include "string_utils.hpp"
#include "util.hpp"
#include "wrappers.hpp"
#include <concepts>
#include <memory>
#include <shared/result.h>
#include <vector>

class Object {
public:
  virtual ClassificationResult classify(Attached &) = 0;
  virtual ExtractionResult extract(Attached &) = 0;
  virtual ~Object() = default;

  explicit Object(int page_num, Attached &) : page_num_(page_num) {}

protected:
  int page_num() { return page_num_; }

private:
  int page_num_ = 0;
};

template <typename... Capabilities>
  requires(std::derived_from<Capabilities, Capability> && ...)
class ObjectWith : public Object, public Capabilities... {
public:
  using Object::Object;

  /// Evaluates every capability, in the order listed, then hands any failures
  /// to evaluate_capability_failures() - which is only called if there are any.
  ClassificationResult evaluate_capabilities(Attached &att) {
    std::vector<CapabilityFailure> failures;
    (collect(failures, Capabilities::evaluate_capability(att)), ...);

    if (failures.empty())
      return ClassificationResult::ok();
    return evaluate_capability_failures(failures);
  }

protected:
  /// In the case that a capability throws an error, users may handle said
  /// errors through this method Or they may choose to omit said errors
  /// entirely.
  ///
  /// For instance, if text extraction fails, then perhaps the user will opt to
  /// skip this page.
  ///
  /// Capabilities MUST be utilized through this class, so CapabilityFailures
  /// may actually propogate, users may utilize the Object class to circumvent
  /// utilizing Capabilities altogether.
  virtual ClassificationResult evaluate_capability_failures(
      const std::vector<CapabilityFailure> &failures) = 0;

private:
  static void collect(std::vector<CapabilityFailure> &into,
                      std::vector<CapabilityFailure> from) {
    for (CapabilityFailure &failure : from)
      into.push_back(std::move(failure));
  }
};

template <class T>
concept HasCapabilities = requires(T &obj, Attached &att) {
  { obj.evaluate_capabilities(att) } -> std::same_as<ClassificationResult>;
};

template <class T>
ClassificationResult evaluate_capabilities_if_any(T &obj, Attached &att) {
  if constexpr (HasCapabilities<T>)
    return obj.evaluate_capabilities(att);
  else
    return ClassificationResult::ok();
}

/// Registers [Type] as the object [name]: generates classify_<name>,
/// extract_<name> and deleter_<Type>, the C ABI functions the generated
/// function map dispatches to. Place it after the class definition, in the
/// class's header - the functions are inline, so every file including the
/// header (the function map, tests) shares a single definition.
#define DEFINE_OBJECT(name, Type)                                              \
                                                                               \
  inline void deleter_##Type(void *p) noexcept {                               \
    delete static_cast<Type *>(p);                                             \
  }                                                                            \
                                                                               \
  inline Result *classify_##name(uint32_t page_num, fz_context *ctx,           \
                                 fz_document *doc) {                           \
                                                                               \
    static_assert(std::is_base_of_v<Object, Type>,                             \
                  "...must derive from Object");                               \
    static_assert(std::is_constructible_v<Type, uint32_t, Attached &>,         \
                  "...needs a (uint32_t page, Attached &) constructor");       \
    static_assert(!std::is_abstract_v<Type>,                                   \
                  "...must implement classify(), extract() and, for "          \
                  "ObjectWith, evaluate_capability_failures()");               \
                                                                               \
    Attached att(ctx, doc, page_num);                                          \
    auto obj = std::make_unique<Type>(page_num, att);                          \
    {                                                                          \
      ClassificationResult caps_evaluated =                                    \
          evaluate_capabilities_if_any(*obj, att);                             \
      if (!caps_evaluated.is_ok())                                             \
        return Result::fail(caps_evaluated.failure());                         \
      ClassificationResult out = obj->classify(att);                           \
      if (!out.is_ok())                                                        \
        return Result::fail(out.failure());                                    \
    }                                                                          \
    return Result::ok(obj.release(), &deleter_##Type);                         \
  }                                                                            \
                                                                               \
  inline Result *extract_##name(uint32_t page_num, fz_context *ctx,            \
                                fz_document *doc, void *shared) {              \
    Type *obj = static_cast<Type *>(shared);                                   \
    Attached att(ctx, doc, page_num);                                          \
    ExtractionResult out = obj->extract(att);                                  \
    if (!out.is_ok())                                                          \
      return Result::fail(out.failure());                                      \
    return Result::ok(new std::string(std::move(out).take_data()),             \
                      &deleter_StdString);                                     \
  }                                                                            \
  static_assert(true, "")
