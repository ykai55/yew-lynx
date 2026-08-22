# Lynx tools_shared patch series

This directory contains JNI generator changes applied to the tools_shared
checkout materialized by the pinned Lynx source.

## Base Revision

- Upstream: <https://github.com/lynx-family/tools-shared>
- Commit: `bdea62f7b500026aab237b271abc7eff279a5c2d`

The patch adds a per-Java-class allowlist for native-renderer-only JNI method
registration. Apply it only after Lynx dependency synchronization has
materialized `third_party/lynx/tools_shared`, and reverse it before returning
the nested checkout to callers. The maintained series contains
`0001-Add-native-renderer-only-JNI-method-filter.patch`; pinning both the nested
revision and patch makes generated JNI filtering reproducible while leaving
stock JNI generation unchanged.

`scripts/verify.sh` clean-applies and reverses this series. Android product
builds apply it before generating the opt-in
`org.lynxsdk.lynx:lynx-native-renderer:0.0.1-0df14207` artifact; it is not a
runtime dependency of either Maven product.
