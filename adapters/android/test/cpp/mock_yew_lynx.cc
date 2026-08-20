#include "yew_lynx.h"

#include <cstdlib>
#include <cstring>

namespace {

constexpr uint8_t kSuccess[] = {20, 0, 0, 0, 'L', 'E', 'B', '2', 0, 255};
constexpr uint8_t kInvalidArgument[] = {20, 0, 0, 0, 'L', 'E', 'B', '2', 1};
constexpr uint8_t kInvalidSession[] = {20, 0, 0, 0, 'L', 'E', 'B', '2', 2};

constexpr YewLynxSession kSession = 41;
bool g_mounted = false;

template <size_t Size>
YewLynxBuffer Copy(const uint8_t (&bytes)[Size]) {
  auto* data = static_cast<uint8_t*>(std::malloc(Size));
  if (data == nullptr) {
    return {nullptr, 0};
  }
  std::memcpy(data, bytes, Size);
  return {data, Size};
}

YewLynxBuffer Copy(const uint8_t* bytes, size_t size) {
  auto* data = static_cast<uint8_t*>(std::malloc(size));
  if (data == nullptr) {
    return {nullptr, 0};
  }
  std::memcpy(data, bytes, size);
  return {data, size};
}

}  // namespace

extern "C" YewLynxMountResult yew_lynx_mount(uint32_t root_id) {
  if (g_mounted || root_id != UINT32_MAX) {
    return {0, Copy(kInvalidArgument)};
  }
  g_mounted = true;
  return {kSession, Copy(kSuccess)};
}

extern "C" YewLynxBuffer yew_lynx_dispatch(
    YewLynxSession session, const uint8_t* event, size_t event_len) {
  if (!g_mounted || session != kSession) {
    return Copy(kInvalidSession);
  }
  if (event == nullptr || event_len < 8 || std::memcmp(event + 4, "LEB2", 4) != 0) {
    return Copy(kInvalidArgument);
  }
  return Copy(kSuccess);
}

extern "C" YewLynxBuffer yew_lynx_complete(
    YewLynxSession session, const uint8_t* response, size_t response_len) {
  if (!g_mounted || session != kSession || response == nullptr || response_len < 8
      || std::memcmp(response + 4, "LEB2", 4) != 0) {
    return Copy(kInvalidArgument);
  }
  return Copy(response, response_len);
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
