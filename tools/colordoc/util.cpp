#include "util.hpp"

void deleter_StdString(void* ptr) { delete static_cast<std::string*>(ptr); }

Result* json_to_payload(nlohmann::json json_obj) {
  std::string* payload = new std::string(json_obj.dump());
  return Result::ok(static_cast<void*>(payload), deleter_StdString);
}
