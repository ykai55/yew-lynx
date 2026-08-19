import { mkdir, readFile, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

import tasm from "@lynx-js/tasm";
import { build } from "esbuild";

const arguments_ = process.argv.slice(2);
if (arguments_.some((argument) => argument !== "--wasm")) {
  throw new Error(`unsupported argument: ${arguments_.join(" ")}`);
}
const forceWasm = arguments_.includes("--wasm");
const adapterDirectory = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const outputDirectory = path.join(adapterDirectory, "dist");
const templateDirectory = path.join(adapterDirectory, "template");
const templateConfig = JSON.parse(
  await readFile(path.join(templateDirectory, "template.config.json"), "utf8"),
);

const bundle = await build({
  bundle: true,
  entryPoints: [path.join(templateDirectory, "shell.mts")],
  format: "iife",
  legalComments: "none",
  platform: "neutral",
  target: "es2015",
  write: false,
});
if (bundle.outputFiles.length !== 1) {
  throw new Error(`expected one bundled MTS script, received ${bundle.outputFiles.length}`);
}

const shell = `${bundle.outputFiles[0].text}\nfunction processData(data) { return data; }\n`;
if (/^\s*(?:import|export)\s/m.test(shell)) {
  throw new Error("bundled MTS script still contains an ESM import or export");
}

const encoderInput = {
  ...templateConfig,
  compilerOptions: {
    bundleModuleMode: "ReturnByFunction",
    qjsCheck: false,
    skipEncode: false,
    useLepusNG: true,
    ...templateConfig.compilerOptions,
  },
  css: {
    cssMap: {},
    cssSource: {},
  },
  lepusCode: {
    root: shell,
  },
  manifest: {},
};

await mkdir(outputDirectory, { recursive: true });
await writeFile(path.join(outputDirectory, "shell.js"), shell, "utf8");
await writeFile(
  path.join(outputDirectory, "template-input.json"),
  `${JSON.stringify(encoderInput, null, 2)}\n`,
  "utf8",
);

const useNapi = !forceWasm && tasm.supportNapi();
const encoded = useNapi
  ? await tasm.encode_napi(encoderInput)
  : await tasm.encode_wasm(encoderInput);
if (encoded.status !== 0 || !Buffer.isBuffer(encoded.buffer) || encoded.buffer.length === 0) {
  throw new Error(encoded.error_msg || "@lynx-js/tasm returned an empty template bundle");
}
const decoded = useNapi
  ? tasm.decode_napi(encoded.buffer)
  : await tasm.decode_wasm(encoded.buffer);
if (decoded["context-type"] !== 1 || decoded["is-lepusng-binary"] !== true) {
  throw new Error("encoded template is not an ordinary LepusNG bundle");
}
await writeFile(path.join(outputDirectory, "yew-lynx-counter.lynx.bundle"), encoded.buffer);

console.log(`Wrote ${path.relative(adapterDirectory, outputDirectory)}/shell.js`);
console.log(`Wrote ${path.relative(adapterDirectory, outputDirectory)}/template-input.json`);
console.log(`Wrote ${path.relative(adapterDirectory, outputDirectory)}/yew-lynx-counter.lynx.bundle`);
