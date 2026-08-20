# Lynx ByteArray patch series

This directory contains the minimal public Lynx change required by protocol v2.
It exposes a Java `byte[]` received by ordinary LepusNG/MTS as a read-only byte
view with `length` and numeric index access, and preserves QuickJS `ArrayBuffer`
arguments as Java `byte[]` when invoking a native module. The bridge can
therefore pass FlatBuffers in both directions without Base64 or string
conversion.

## Base revision

- Upstream: <https://github.com/lynx-family/lynx>
- Commit: `0df14207cebb060f1bed8de12b64a1119dee8f06`

Apply patches in the order listed by `series`. Other Lynx revisions require a
rebase and complete reverification.

## Verification

The patch includes focused `QuickContextSourceBundleTest` coverage for length,
all unsigned byte values, out-of-range access, and ArrayBuffer-to-ByteArray
conversion. From a clean checkout of the base revision:

```bash
while IFS= read -r patch; do
  git am "/path/to/lynx-element-bridge/patches/lynx/$patch"
done < /path/to/lynx-element-bridge/patches/lynx/series
```

Run Lynx's `runtime_tests_exec` target and select
`QuickContextSourceBundleTest.ExposesByteArrayAsReadableBytes`. The repository
verification also checks that the patch applies cleanly to the pinned submodule.
