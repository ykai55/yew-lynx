import { build } from "esbuild";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");

await build({
  entryPoints: [resolve(root, "src/wire.mts")],
  outfile: resolve(root, "src/wire-generated.js"),
  bundle: true,
  format: "esm",
  platform: "neutral",
  mainFields: ["module", "main"],
  nodePaths: [resolve(root, "node_modules")],
  target: "es2015",
  inject: [resolve(root, "src/text-codec.mts")],
  legalComments: "none",
});
