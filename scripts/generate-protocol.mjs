#!/usr/bin/env node

import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { existsSync, mkdirSync, readFileSync, readdirSync, rmSync, writeFileSync } from "node:fs";
import { dirname, join, relative, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const LYNX_REVISION = "0df14207cebb060f1bed8de12b64a1119dee8f06";
const FLATC_VERSION = "25.2.10";
const SOURCE = join(
  ROOT,
  "third_party/lynx/js_libraries/type-element-api/types/element-api.d.ts",
);
const BINDINGS = join(
  ROOT,
  "third_party/lynx/core/runtime/lepusng/bindings/renderer_ng.cc",
);
const BINDINGS_HEADER = join(
  ROOT,
  "third_party/lynx/core/runtime/lepus/bindings/renderer.h",
);
const SCHEMA = join(ROOT, "protocol/schema/element_bridge_v2.fbs");
const MANIFEST = join(ROOT, "protocol/capabilities/0df14207.json");
const GENERATED = join(ROOT, "protocol/generated");
const CHECK = process.argv.includes("--check");
const FLATC = process.env.FLATC || "flatc";

function splitTopLevel(value, delimiter = ",") {
  const parts = [];
  const pairs = new Map([["(", ")"], ["[", "]"], ["{", "}"], ["<", ">"]]);
  const stack = [];
  let start = 0;
  let quote = null;

  for (let index = 0; index < value.length; index += 1) {
    const character = value[index];
    if (quote !== null) {
      if (character === "\\") {
        index += 1;
      } else if (character === quote) {
        quote = null;
      }
      continue;
    }
    if (character === "\"" || character === "'" || character === "`") {
      quote = character;
    } else if (pairs.has(character)) {
      stack.push(pairs.get(character));
    } else if (stack.at(-1) === character) {
      stack.pop();
    } else if (character === delimiter && stack.length === 0) {
      parts.push(value.slice(start, index).trim());
      start = index + 1;
    }
  }
  const final = value.slice(start).trim();
  if (final !== "") {
    parts.push(final);
  }
  return parts;
}

function extractFunctions(source) {
  const functions = [];
  const expression = /\bfunction\s+([_$A-Za-z][_$A-Za-z0-9]*)\s*\(/g;
  let match;
  while ((match = expression.exec(source)) !== null) {
    let cursor = expression.lastIndex;
    let depth = 1;
    let quote = null;
    while (cursor < source.length && depth > 0) {
      const character = source[cursor];
      if (quote !== null) {
        if (character === "\\") {
          cursor += 1;
        } else if (character === quote) {
          quote = null;
        }
      } else if (character === "\"" || character === "'" || character === "`") {
        quote = character;
      } else if (character === "(") {
        depth += 1;
      } else if (character === ")") {
        depth -= 1;
      }
      cursor += 1;
    }
    assert.equal(depth, 0, `unterminated declaration for ${match[1]}`);
    const parameterText = source.slice(expression.lastIndex, cursor - 1);
    const returnStart = source.indexOf(":", cursor);
    assert.notEqual(returnStart, -1, `missing return type for ${match[1]}`);
    let declarationEnd = returnStart + 1;
    let returnDepth = 0;
    let returnQuote = null;
    while (declarationEnd < source.length) {
      const character = source[declarationEnd];
      if (returnQuote !== null) {
        if (character === "\\") {
          declarationEnd += 1;
        } else if (character === returnQuote) {
          returnQuote = null;
        }
      } else if (character === "\"" || character === "'" || character === "`") {
        returnQuote = character;
      } else if ("([{<".includes(character)) {
        returnDepth += 1;
      } else if (")]}>".includes(character)) {
        returnDepth -= 1;
      } else if (returnDepth === 0 && character === ";") {
        break;
      } else if (
        returnDepth === 0
        && character === "\n"
        && /^\n\s*function\b/.test(source.slice(declarationEnd))
      ) {
        break;
      }
      declarationEnd += 1;
    }
    assert.ok(declarationEnd < source.length, `unterminated return type for ${match[1]}`);
    const returnType = source.slice(returnStart + 1, declarationEnd).replace(/\s+/g, " ").trim();
    const hasSemicolon = source[declarationEnd] === ";";
    const declaration = source
      .slice(match.index, declarationEnd + Number(hasSemicolon))
      .replace(/\s+/g, " ")
      .trim();
    const parameters = splitTopLevel(parameterText).map((parameter) => {
      const separator = parameter.indexOf(":");
      assert.notEqual(separator, -1, `missing parameter type in ${match[1]}: ${parameter}`);
      const rawName = parameter.slice(0, separator).trim();
      return {
        name: rawName.replace(/\?$/, ""),
        optional: rawName.endsWith("?"),
        type: parameter.slice(separator + 1).replace(/\s+/g, " ").trim(),
      };
    });
    functions.push({
      name: match[1],
      declaration,
      parameters,
      returnType,
    });
    expression.lastIndex = declarationEnd + Number(hasSemicolon);
  }
  return functions;
}

function pascalCase(name) {
  return name
    .replace(/^_+/, "")
    .replace(/[^A-Za-z0-9]+(.)/g, (_, next) => next.toUpperCase())
    .replace(/^[a-z]/, (first) => first.toUpperCase());
}

function snakeCase(name) {
  const value = name
    .replace(/([a-z0-9])([A-Z])/g, "$1_$2")
    .replace(/[^A-Za-z0-9]+/g, "_")
    .replace(/^_+|_+$/g, "")
    .toLowerCase();
  return value === "type" ? "value_type" : value;
}

function camelCase(name) {
  return name.replace(/_([a-z0-9])/g, (_, next) => next.toUpperCase());
}

function capabilityName(name) {
  return snakeCase(name.replace(/^_+/, ""));
}

function isCallback(parameter) {
  return /Callback\b|=>/.test(parameter.type)
    || /callback|func|onLayoutReady|componentAtIndex|enqueueComponent/i.test(parameter.name);
}

function fbsType(parameter) {
  const type = parameter.type;
  const name = parameter.name;
  if (isCallback(parameter)) {
    return "uint";
  }
  if (/ElementRef/.test(type)) {
    if (/ElementRef\[\]|Array<ElementRef>|ElementRef\s*\|\s*ElementRef\[\]/.test(type)) {
      return "ElementReferences";
    }
    return "uint";
  }
  if (/\bstring\[\]|Array<string>/.test(type)) {
    return "[string]";
  }
  if (/\bnumber\[\]|Array<number>/.test(type)) {
    return "[double]";
  }
  if (/^string(?:\s*\|\s*(?:null|undefined))*$/.test(type)) {
    return "string";
  }
  if (/^boolean(?:\s*\|\s*undefined)?$/.test(type)) {
    return "bool";
  }
  if (/^number(?:\s*\|\s*undefined)?$/.test(type) || /^AnimationOperation(?:\.|$)/.test(type)) {
    return /(?:^|_)(?:id|index|count)$/i.test(snakeCase(name)) || /ID|Id|Index|Count|css/i.test(name)
      ? "uint"
      : "double";
  }
  return "Payload";
}

function resultKind(returnType) {
  if (returnType === "void") return "void";
  if (/ElementRef\[\]/.test(returnType)) return "element_ids";
  if (/ElementRef/.test(returnType)) return "element_id";
  if (/^string\[\]$/.test(returnType)) return "strings";
  if (/^string$/.test(returnType)) return "string";
  if (/^boolean$/.test(returnType)) return "boolean";
  if (/^number/.test(returnType)) return "number";
  return "payload";
}

function schemaFor(functions) {
  const tables = functions.map((api) => {
    const fields = api.parameters.map((parameter, index) => {
      const type = fbsType(parameter);
      const field = snakeCase(parameter.name);
      const optionalDefault = parameter.optional && ["uint", "double", "bool"].includes(type)
        ? " = null"
        : "";
      return `  ${field}:${type}${optionalDefault} (id: ${index});`;
    });
    return [
      `// ${api.declaration}`,
      `table ${pascalCase(api.name)}Command {`,
      ...fields,
      "}",
    ].join("\n");
  }).join("\n\n");
  const unionMembers = functions.map((api) => `  ${pascalCase(api.name)}Command`).join(",\n");

  return `// Generated by scripts/generate-protocol.mjs. Do not edit.
// Source: lynx ${LYNX_REVISION}/js_libraries/type-element-api/types/element-api.d.ts

namespace Lynx.ElementBridge.V2;

file_identifier "LEB2";
file_extension "leb";

enum Channel : ubyte { NONE = 0, COMMAND = 1, RESULT = 2, EVENT = 3 }
enum Status : ushort {
  OK = 0,
  INVALID_ARGUMENT = 1,
  INVALID_SESSION = 2,
  WRONG_THREAD = 3,
  UNSUPPORTED = 4,
  INVALID_OWNERSHIP = 5,
  INVALID_LISTENER = 6,
  RESOURCE_EXHAUSTED = 7,
  HOST_ERROR = 8,
  PANIC = 9,
  INTERNAL_ERROR = 10
}
enum ReferenceCardinality : ubyte { NONE = 0, ONE = 1, MANY = 2 }
enum ResultValueKind : ubyte {
  NONE = 0,
  ELEMENT_ID = 1,
  ELEMENT_IDS = 2,
  STRING = 3,
  STRINGS = 4,
  BOOLEAN = 5,
  NUMBER = 6,
  PAYLOAD = 7
}

table Payload {
  content_type:string (id: 0, required);
  bytes:[ubyte] (id: 1);
}

table ElementReferences {
  cardinality:ReferenceCardinality (id: 0);
  one:uint (id: 1);
  many:[uint] (id: 2);
}

${tables}

// Bridge lifecycle operation; not part of the public Element API declaration count.
table ReleaseElementCommand {
  node:uint (id: 0);
}

union ElementCommand {
${unionMembers},
  ReleaseElementCommand
}

table CapabilityRequest {
  name:string (id: 0, required);
  required:bool (id: 1);
}

table CreateSessionRequest {
  root_id:uint (id: 0);
  capabilities:[CapabilityRequest] (id: 1);
}

table Command {
  result_slot:uint = 4294967295 (id: 0);
  result_node_id:uint (id: 1);
  result_node_ids:[uint] (id: 2);
  listener_id:uint (id: 3);
  operation:ElementCommand (id: 5);
}

table CommandBatch {
  session_id:uint (id: 0);
  sequence:uint (id: 1);
  commands:[Command] (id: 2);
  final_commit:bool = true (id: 3);
}

table DestroySessionRequest {
  session_id:uint (id: 0);
}

table ElementIdResult { value:uint (id: 0); }
table ElementIdsResult { values:[uint] (id: 0); }
table StringResult { value:string (id: 0); }
table StringsResult { values:[string] (id: 0); }
table BooleanResult { value:bool (id: 0); }
table NumberResult { value:double (id: 0); }

union ResultValue {
  ElementIdResult,
  ElementIdsResult,
  StringResult,
  StringsResult,
  BooleanResult,
  NumberResult,
  Payload
}

table ResultItem {
  slot:uint (id: 0);
  status:Status (id: 1);
  message:string (id: 2);
  value_kind:ResultValueKind (id: 3);
  value:ResultValue (id: 5);
}

table ResponseBatch {
  session_id:uint (id: 0);
  sequence:uint (id: 1);
  status:Status (id: 2);
  message:string (id: 3);
  results:[ResultItem] (id: 4);
  committed:bool (id: 5);
}

table EventMessage {
  session_id:uint (id: 0);
  listener_id:uint (id: 1);
  callback_id:uint (id: 2);
  content_type:string (id: 3, required);
  payload:[ubyte] (id: 4);
}

union Message {
  CreateSessionRequest,
  CommandBatch,
  DestroySessionRequest,
  ResponseBatch,
  EventMessage
}

table Envelope {
  version:ushort = 2 (id: 0);
  channel:Channel (id: 1);
  message:Message (id: 3);
}

root_type Envelope;
`;
}

function manifestFor(functions, bindingsSource, bindingsHeader) {
  const constants = new Map();
  const constantExpression = /(?:constexpr\s+)?(?:static\s+)?const\s+char\s*\*\s*([A-Za-z0-9_]+)\s*=\s*"([_$A-Za-z0-9]+)"/g;
  let constant;
  while ((constant = constantExpression.exec(bindingsHeader)) !== null) {
    constants.set(constant[2], constant[1]);
  }
  const capabilities = functions.map((api, index) => ({
    id: index + 1,
    name: capabilityName(api.name),
    declarationName: api.name,
    declaration: api.declaration,
    parameters: api.parameters.map((parameter) => ({
      ...parameter,
      wireType: fbsType(parameter),
    })),
    returnType: api.returnType,
    resultKind: resultKind(api.returnType),
    fiberBinding: constants.has(api.name) && bindingsSource.includes(constants.get(api.name)),
    android: constants.has(api.name) && bindingsSource.includes(constants.get(api.name))
      ? "available"
      : "unsupported",
  }));
  return {
    product: "lynx-element-bridge",
    protocolVersion: 2,
    lynxRevision: LYNX_REVISION,
    source: "js_libraries/type-element-api/types/element-api.d.ts",
    declarationCount: capabilities.length,
    nativeOnlyBindingsInScope: false,
    capabilities,
  };
}

function rustCapabilities(manifest) {
  const entries = manifest.capabilities.map((capability) => [
    "    GeneratedCapability {",
    `        id: ${capability.id},`,
    `        name: "${capability.name}",`,
    `        declaration_name: "${capability.declarationName}",`,
    `        available: ${capability.android === "available"},`,
    "    },",
  ].join("\n")).join("\n");
  return `// Generated by scripts/generate-protocol.mjs. Do not edit.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GeneratedCapability {
    pub id: u16,
    pub name: &'static str,
    pub declaration_name: &'static str,
    pub available: bool,
}

pub const LYNX_REVISION: &str = "${LYNX_REVISION}";
pub const CAPABILITIES: &[GeneratedCapability] = &[
${entries}
];
`;
}

function typescriptElementDispatcher(functions, manifest, generatedPath) {
  const typescriptRoot = join(generatedPath, "typescript");
  const classFiles = new Map();
  for (const file of readdirSync(typescriptRoot, { recursive: true })) {
    if (typeof file !== "string" || !file.endsWith(".ts")) continue;
    const source = readFileSync(join(typescriptRoot, file), "utf8");
    for (const match of source.matchAll(/export class ([A-Za-z0-9_]+)/g)) {
      classFiles.set(match[1], file.replace(/\.ts$/, ".js"));
    }
  }
  const classes = functions.map((api) => `${pascalCase(api.name)}Command`);
  const imports = classes.map((className) => {
    const file = classFiles.get(className);
    assert.ok(file, `missing generated TypeScript class ${className}`);
    return `import { ${className} } from "./${file}";`;
  });
  const commandFile = classFiles.get("Command");
  assert.ok(commandFile, "missing generated TypeScript Command class");
  const enumFile = join("lynx/element-bridge/v2/element-command.js");
  const cases = functions.map((api, index) => {
    const className = classes[index];
    const capability = manifest.capabilities[index];
    const args = api.parameters.map((parameter) => {
      const accessor = camelCase(snakeCase(parameter.name));
      const wireType = fbsType(parameter);
      let value;
      if (wireType === "Payload") {
        value = `decodePayload(operation.${accessor}())`;
      } else if (wireType === "ElementReferences") {
        value = `decodeReferences(operation.${accessor}())`;
      } else if (wireType.startsWith("[")) {
        value = `Array.from({ length: operation.${accessor}Length() }, (_, index) => operation.${accessor}(index))`;
      } else {
        value = `operation.${accessor}()`;
      }
      let kind = "value";
      if (isCallback(parameter)) {
        kind = "callback";
      } else if (/ElementRef/.test(parameter.type)) {
        kind = /\[\]|Array<|\|/.test(parameter.type) ? "node_or_nodes" : "node";
      }
      return `{ name: "${parameter.name}", kind: "${kind}", value: ${value} }`;
    });
    return `    case ElementCommand.${className}: {
      const operation = command.operation(new ${className}()) as ${className} | null;
      if (operation === null) throw new TypeError("${api.name} command payload is missing");
      return {
        name: "${api.name}",
        capability: "${capability.name}",
        available: ${capability.android === "available"},
        resultKind: "${capability.resultKind}",
        args: [${args.join(", ")}],
      };
    }`;
  }).join("\n");
  return `// Generated by scripts/generate-protocol.mjs. Do not edit.

import { Command } from "./${commandFile}";
import { ElementCommand } from "./${enumFile}";
${imports.join("\n")}

export type DecodedElementArgument = {
  name: string;
  kind: "value" | "node" | "node_or_nodes" | "callback";
  value: unknown;
};

export type DecodedElementCommand = {
  name: string;
  capability: string;
  available: boolean;
  resultKind: string;
  args: DecodedElementArgument[];
};

export function decodeElementApiCommand(
  command: Command,
  decodePayload: (payload: unknown) => unknown,
  decodeReferences: (references: unknown) => unknown,
): DecodedElementCommand {
  switch (command.operationType()) {
${cases}
    default:
      throw new TypeError(\`unknown Element command \${command.operationType()}\`);
  }
}
`;
}

function write(path, content) {
  mkdirSync(dirname(path), { recursive: true });
  writeFileSync(path, content);
}

function normalizeGeneratedTree(path) {
  for (const entry of readdirSync(path, { withFileTypes: true })) {
    const entryPath = join(path, entry.name);
    if (entry.isDirectory()) {
      normalizeGeneratedTree(entryPath);
      continue;
    }
    const content = readFileSync(entryPath, "utf8")
      .replace(/[\t ]+$/gm, "")
      .replace(/\n+$/, "\n");
    writeFileSync(entryPath, content);
  }
}

function generate(outputRoot) {
  assert.ok(existsSync(SOURCE), `missing pinned Lynx source: ${relative(ROOT, SOURCE)}`);
  const functions = extractFunctions(readFileSync(SOURCE, "utf8"));
  assert.equal(functions.length, 107, "pinned public Element API declaration count changed");
  const bindingsSource = readFileSync(BINDINGS, "utf8");
  const bindingsHeader = readFileSync(BINDINGS_HEADER, "utf8");
  const schemaPath = join(outputRoot, relative(ROOT, SCHEMA));
  const manifestPath = join(outputRoot, relative(ROOT, MANIFEST));
  const generatedPath = join(outputRoot, relative(ROOT, GENERATED));
  write(schemaPath, schemaFor(functions));
  const manifest = manifestFor(functions, bindingsSource, bindingsHeader);
  write(manifestPath, `${JSON.stringify(manifest, null, 2)}\n`);

  const version = execFileSync(FLATC, ["--version"], { encoding: "utf8" }).trim();
  assert.equal(version, `flatc version ${FLATC_VERSION}`, "unexpected flatc version");
  rmSync(generatedPath, { recursive: true, force: true });
  for (const [language, options] of [
    ["rust", ["--rust", "--gen-name-strings"]],
    ["typescript", ["--ts", "--ts-flat-files"]],
    ["java", ["--java"]],
  ]) {
    const destination = join(generatedPath, language);
    mkdirSync(destination, { recursive: true });
    execFileSync(FLATC, [...options, "-o", destination, schemaPath], { stdio: "inherit" });
  }
  write(join(generatedPath, "rust/capabilities_generated.rs"), rustCapabilities(manifest));
  write(
    join(generatedPath, "typescript/element_api_dispatch.ts"),
    typescriptElementDispatcher(functions, manifest, generatedPath),
  );
  normalizeGeneratedTree(generatedPath);
}

if (CHECK) {
  const temporary = join(ROOT, ".deps/protocol-check");
  rmSync(temporary, { recursive: true, force: true });
  generate(temporary);
  for (const path of [
    "protocol/schema",
    "protocol/capabilities",
    "protocol/generated",
  ]) {
    execFileSync("diff", ["-ru", join(temporary, path), join(ROOT, path)], {
      stdio: "inherit",
    });
  }
  rmSync(temporary, { recursive: true, force: true });
} else {
  generate(ROOT);
}
