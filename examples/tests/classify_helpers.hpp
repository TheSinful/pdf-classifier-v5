#pragma once

#include <pdf_classifier_lib/object.hpp>

// Runs an object the way DEFINE_OBJECT does: its capabilities first (for
// ObjectWith<...>), then classify(). Calling classify() directly would skip the
// capabilities and leave e.g. TextExtraction's text empty.
template <class T> ClassificationResult classify_like_engine(T& obj, Attached& att) {
  ClassificationResult capabilities = evaluate_capabilities_if_any(obj, att);
  if (!capabilities.is_ok())
    return capabilities;
  return obj.classify(att);
}
