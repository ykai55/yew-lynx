# lynx-cssc

`lynx-cssc` converts a Lynx `ruleList` JSON array into one raw compiled CSS
fragment. It does not parse CSS text and does not emit a template bundle.

From a clean pinned `third_party/lynx` checkout, apply the maintained patches
in order and build the host targets:

```bash
while IFS= read -r patch; do
  git -C third_party/lynx apply "../../patches/lynx/$patch"
done < patches/lynx/series

cd third_party/lynx
tools/env.sh gn gen out/lynx-cssc \
  --args='enable_unittests=true is_debug=false use_flutter_cxx=false'
tools/env.sh ninja -C out/lynx-cssc \
  lynx-cssc lynx-cssc-tests
```

Compile one rule list:

```bash
out/lynx-cssc/lynx-cssc --input rules.json --output style.lynxcss
```

## Stylist static CSS

The experimental `stylist-lynx-cssc` Cargo tool converts Stylist 0.15.1's
static CSS subset into the `ruleList` consumed by `lynx-cssc`. A complete flow
has three steps.

1. Write Stylist CSS with an explicit, stable application class:

   ```css
   /* styles/counter.css */
   color: red;

   :hover {
     opacity: 0.8;
   }

   .label {
     font-size: 16px;
   }
   ```

2. Convert it to a `ruleList`, then compile the native fragment:

   ```bash
   cargo run --locked -p stylist-lynx-cssc -- \
     --input styles/counter.css --class counter --output styles/counter.rules.json
   third_party/lynx/out/lynx-cssc/lynx-cssc \
     --input styles/counter.rules.json --output styles/counter.lynxcss
   ```

3. Render the same class and embed the fragment at launch:

   ```rust
   #[function_component(App)]
   fn app() -> Html {
       html! { <view class="counter"><text class="label">{"Count"}</text></view> }
   }

   lynx::yew::launch_with_style_sheets!(App, [
       include_bytes!("../styles/counter.lynxcss"),
   ]);
    ```

### Automatic Cargo builds

Applications can run both conversion steps from `build.rs`. Build the pinned
`lynx-cssc` target once, set `LYNX_CSSC` to its absolute path, and add the build
helper:

```toml
[build-dependencies]
lynx-css-build = { path = "../../crates/lynx-css-build" }
```

```rust
// build.rs
fn main() {
    lynx_css_build::compile("styles/counter.css", "counter", "counter.lynxcss")
        .expect("failed to compile counter styles");
}
```

The helper emits Cargo rebuild directives for the CSS input and `LYNX_CSSC`,
writes the generated files under `OUT_DIR`, and fails the build if either the
static Stylist conversion or native compiler fails. Embed its output without
hard-coding the target directory:

```rust
lynx::yew::launch_with_style_sheets!(App, [
    lynx::include_lynx_style_sheet!("counter.lynxcss"),
]);
```

This is not a complete Stylist runtime. It intentionally requires an explicit
class instead of generating Stylist's random class, and rejects interpolation,
at-rules, blocks nested inside selector blocks, empty rules, and anything outside
the static `StyleAttr` subset. Top-level selector blocks such as `.label` and
`:hover` are supported and scoped beneath the explicit class.

Run the raw ruleList and Stylist-to-native CLI smoke tests plus the native
import/decode assertion:

```bash
bash ../../scripts/test-lynx-cssc.sh \
  out/lynx-cssc/lynx-cssc \
  out/lynx-cssc/lynx_cssc_native_test_exec
```

The repository verification applies and checks the Lynx patch series without
building C++ by default. To include the host-tool build and this smoke test, run:

```bash
LYNX_VERIFY_CSSC=1 ./scripts/verify.sh
```

Without the switch, `verify.sh` prints an explicit `lynx-cssc` skip message.

The payload contract is pinned to this repository's Lynx revision and native
CSS profile. Normal Cargo builds do not invoke GN or depend on this tool.

This first version accepts a non-empty array of `StyleRule` objects only. Each
rule requires a string `selectorText.value`, a `style` array whose declarations
contain string `name` and `value` fields, and a `variables` object with string
values. Other top-level Lynx rule types are rejected before invoking the Lynx
CSS parser.
