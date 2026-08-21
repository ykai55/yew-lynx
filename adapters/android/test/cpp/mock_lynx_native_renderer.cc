#include "lynx_native_renderer.h"

namespace {

const LynxNativeRendererApiV1 kApi = {
    LYNX_NATIVE_RENDERER_ABI_VERSION,
    sizeof(LynxNativeRendererApiV1),
    nullptr,
    nullptr,
    nullptr,
    nullptr,
    nullptr,
    nullptr,
    nullptr,
    nullptr,
    nullptr,
    nullptr,
    nullptr,
    nullptr,
    nullptr,
    nullptr,
    nullptr,
};

}  // namespace

extern "C" __attribute__((visibility("default")))
const LynxNativeRendererApiV1* lynx_native_renderer_get_api(
    uint32_t requested_version) {
  return requested_version == LYNX_NATIVE_RENDERER_ABI_VERSION ? &kApi
                                                                : nullptr;
}
