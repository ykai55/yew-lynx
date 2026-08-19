import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

test("the Lynx host uses specialized public constructors and the page component ID", async () => {
  const calls = [];
  const names = [
    "__CreateView",
    "__CreateText",
    "__CreateImage",
    "__CreateScrollView",
    "__CreateElement",
  ];
  const previous = new Map(names.map((name) => [name, globalThis[name]]));
  globalThis.__CreateView = (parentId) => calls.push(["view", parentId]);
  globalThis.__CreateText = (parentId) => calls.push(["text", parentId]);
  globalThis.__CreateImage = (parentId) => calls.push(["image", parentId]);
  globalThis.__CreateScrollView = (parentId) => calls.push(["scroll-view", parentId]);
  globalThis.__CreateElement = (tag, parentId) => calls.push(["element", tag, parentId]);

  try {
    const source = await readFile(new URL("../src/lynx-fiber-host.mts", import.meta.url), "utf8");
    const module = await import(`data:text/javascript;base64,${Buffer.from(source).toString("base64")}`);
    const host = module.createLynxFiberHost(73);

    host.createElement("view");
    host.createElement("text");
    host.createElement("image");
    host.createElement("scroll-view");
    host.createElement("input");

    assert.deepEqual(calls, [
      ["view", 73],
      ["text", 73],
      ["image", 73],
      ["scroll-view", 73],
      ["element", "input", 73],
    ]);
  } finally {
    for (const [name, value] of previous) {
      if (value === undefined) {
        delete globalThis[name];
      } else {
        globalThis[name] = value;
      }
    }
  }
});
