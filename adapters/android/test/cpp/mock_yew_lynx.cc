#include "yew_lynx.h"

#include <cstdlib>
#include <cstring>
#include <string_view>

namespace {

constexpr char kSuccess[] =
    "{\"version\":1,\"ok\":true,\"operations\":[{\"op\":\"flush\",\"root\":9007199254740991}]}";
constexpr char kInvalidArgument[] =
    "{\"version\":1,\"ok\":false,\"status\":1,\"error\":\"invalid_argument\",\"operations\":[]}";
constexpr char kInvalidSession[] =
    "{\"version\":1,\"ok\":false,\"status\":3,\"error\":\"invalid_session\",\"operations\":[]}";

constexpr YewLynxSession kSession = 41;
bool g_mounted = false;

YewLynxBuffer Copy(std::string_view json) {
  auto* data = static_cast<uint8_t*>(std::malloc(json.size()));
  if (data == nullptr) {
    return {nullptr, 0};
  }
  std::memcpy(data, json.data(), json.size());
  return {data, json.size()};
}

bool Equals(const uint8_t* data, size_t length, std::string_view expected) {
  return data != nullptr && length == expected.size()
      && std::memcmp(data, expected.data(), length) == 0;
}

}  // namespace

extern "C" YewLynxMountResult yew_lynx_mount(const uint8_t* root_id,
                                               size_t root_id_len) {
  if (g_mounted || !Equals(root_id, root_id_len, "9007199254740991")) {
    return {0, Copy(kInvalidArgument)};
  }
  g_mounted = true;
  return {kSession, Copy(kSuccess)};
}

extern "C" YewLynxBuffer yew_lynx_dispatch(
    YewLynxSession session, const uint8_t* listener_id,
    size_t listener_id_len, const uint8_t* event_name,
    size_t event_name_len) {
  if (!g_mounted || session != kSession) {
    return Copy(kInvalidSession);
  }
  if (!Equals(listener_id, listener_id_len, "9007199254740991")
      || !Equals(event_name, event_name_len, "tap")) {
    return Copy(kInvalidArgument);
  }
  return Copy(kSuccess);
}

extern "C" YewLynxDestroyResult yew_lynx_destroy(
    YewLynxSession session) {
  if (!g_mounted || session != kSession) {
    return {0, Copy(kInvalidSession)};
  }
  g_mounted = false;
  return {1, Copy(kSuccess)};
}

extern "C" void yew_lynx_buffer_free(YewLynxBuffer buffer) {
  std::free(buffer.data);
}
