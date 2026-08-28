#ifndef LYNX_WAMR_APPLICATION_H_
#define LYNX_WAMR_APPLICATION_H_

#include <stddef.h>
#include <stdint.h>

#include "lynx_native_application.h"

#ifdef __cplusplus
extern "C" {
#endif

LynxElementBridgeNativeMountResult lynx_element_bridge_wamr_mount(
    LynxNativeRendererGetApiFn get_api,
    LynxNativeHostHandle host,
    const uint8_t* module_data,
    size_t module_len);

LynxNativeRendererStatus lynx_element_bridge_wamr_replace(
    LynxElementBridgeSession session,
    const uint8_t* module_data,
    size_t module_len);

LynxElementBridgeNativeDestroyResult lynx_element_bridge_wamr_destroy(
    LynxElementBridgeSession session);

#ifdef __cplusplus
}  /* extern "C" */
#endif

#endif  /* LYNX_WAMR_APPLICATION_H_ */
