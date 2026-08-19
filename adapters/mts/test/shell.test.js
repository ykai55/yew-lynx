import assert from "node:assert/strict";
import test from "node:test";

import { installYewLynxShell } from "../src/shell-core.js";

function createEmitter() {
  const listeners = new Map();
  return {
    addEventListener(name, listener) {
      assert.equal(listeners.has(name), false, `duplicate ${name} listener`);
      listeners.set(name, listener);
    },
    emit(name, data = []) {
      const listener = listeners.get(name);
      assert.ok(listener, `missing ${name} listener`);
      return listener({ data });
    },
  };
}

function createShellHarness(brokers) {
  const engine = createEmitter();
  const native = createEmitter();
  const calls = [];
  const lynxApi = {
    getEngine: () => engine,
    getNative: () => native,
    module(name) {
      calls.push(["module", name]);
      return { name };
    },
    reportError(error) {
      calls.push(["reportError", error]);
    },
  };
  installYewLynxShell({
    lynxApi,
    createPage(componentId, cssId) {
      calls.push(["createPage", componentId, cssId]);
      return { page: calls.length };
    },
    getElementUniqueId(page) {
      calls.push(["getElementUniqueId", page]);
      return 73;
    },
    createHost(parentId) {
      calls.push(["createHost", parentId]);
      return { parentId };
    },
    brokerFactory(options) {
      calls.push(["broker", options]);
      const broker = brokers.shift();
      assert.ok(broker, "missing broker fixture");
      return broker;
    },
  });
  return { engine, native, calls };
}

function brokerFixture({ mountError, destroyError } = {}) {
  const calls = [];
  return {
    calls,
    mount() {
      calls.push("mount");
      if (mountError !== undefined) {
        throw mountError;
      }
    },
    destroy() {
      calls.push("destroy");
      if (destroyError !== undefined) {
        throw destroyError;
      }
    },
  };
}

test("shell mounts with a fresh page ID and publishes only the mounted broker", () => {
  const first = brokerFixture();
  const harness = createShellHarness([first]);

  harness.engine.emit("__RenderPage", [{}, {
    initPage: null,
    cacheData: [],
    hydrateMap: null,
  }]);

  assert.deepEqual(first.calls, ["mount"]);
  const options = harness.calls.find(([name]) => name === "broker")[1];
  assert.equal(options.host.parentId, 73);
  assert.equal(options.rootId, 1);
  assert.equal(options.suppressMountFlush, true);
  assert.deepEqual(harness.calls.filter(([name]) => name === "module"), [["module", "YewLynx"]]);
  assert.throws(
    () => harness.engine.emit("__RenderPage", [{}, {}]),
    /already mounted/,
  );
});

test("shell best-effort destroys a failed mount and permits a later mount", () => {
  const mountError = new Error("mount failed");
  const first = brokerFixture({ mountError, destroyError: new Error("cleanup failed") });
  const second = brokerFixture();
  const harness = createShellHarness([first, second]);

  assert.throws(
    () => harness.engine.emit("__RenderPage", [{}, {}]),
    (error) => error === mountError,
  );
  assert.deepEqual(first.calls, ["mount", "destroy"]);

  harness.engine.emit("__RenderPage", [{}, {}]);
  assert.deepEqual(second.calls, ["mount"]);
});

test("shell rejects cached roots, cache data, hydrate maps, and SSR", () => {
  const harness = createShellHarness([]);
  const invalidOptions = [
    { initPage: {} },
    { cacheData: [{}] },
    { cacheData: { data: {} } },
    { hydrateMap: { root: 1 } },
  ];

  for (const options of invalidOptions) {
    assert.throws(
      () => harness.engine.emit("__RenderPage", [{}, options]),
      /does not support/,
    );
  }
  assert.throws(() => harness.engine.emit("__SSRHydrate", ["hydrate", []]), /does not support SSR/);
  assert.equal(harness.calls.some(([name]) => name === "createPage"), false);
});

test("shell handles ordinary updates, component removal, reload remount, and lifetime destroy", () => {
  const first = brokerFixture();
  const second = brokerFixture();
  const third = brokerFixture();
  const harness = createShellHarness([first, second, third]);

  harness.engine.emit("__RenderPage", [{}, {}]);
  harness.engine.emit("__UpdatePage", [{ count: 1 }, {}]);
  assert.deepEqual(first.calls, ["mount"]);

  harness.engine.emit("__RemoveComponents");
  assert.deepEqual(first.calls, ["mount", "destroy"]);
  harness.engine.emit("__UpdatePage", [{}, { reloadTemplate: true }]);
  assert.deepEqual(second.calls, ["mount"]);

  harness.engine.emit("__UpdatePage", [{}, { reloadFromJS: true }]);
  assert.deepEqual(second.calls, ["mount", "destroy"]);
  assert.deepEqual(third.calls, ["mount"]);
  assert.deepEqual(
    harness.calls.filter(([name]) => name === "broker").map(([, options]) => options.suppressMountFlush),
    [true, false, false],
  );

  harness.native.emit("__DestroyLifetime", [1]);
  assert.deepEqual(third.calls, ["mount", "destroy"]);
});

test("reload reports cleanup failure without blocking remount", () => {
  const cleanupError = new Error("destroy failed");
  cleanupError.status = 9;
  const first = brokerFixture({ destroyError: cleanupError });
  const second = brokerFixture();
  const harness = createShellHarness([first, second]);

  harness.engine.emit("__RenderPage", [{}, {}]);
  harness.engine.emit("__RemoveComponents");
  harness.engine.emit("__UpdatePage", [{}, { reloadTemplate: true }]);

  assert.deepEqual(first.calls, ["mount", "destroy"]);
  assert.deepEqual(second.calls, ["mount"]);
  assert.deepEqual(
    harness.calls.filter(([name]) => name === "reportError"),
    [["reportError", cleanupError]],
  );
});

test("shell rejects an unsafe page component ID before creating a broker", () => {
  const engine = createEmitter();
  const native = createEmitter();
  let brokerCreated = false;
  installYewLynxShell({
    lynxApi: {
      getEngine: () => engine,
      getNative: () => native,
      module: () => ({}),
      reportError: () => {},
    },
    createPage: () => ({}),
    getElementUniqueId: () => Number.MAX_SAFE_INTEGER + 1,
    createHost: () => ({}),
    brokerFactory: () => {
      brokerCreated = true;
      return brokerFixture();
    },
  });

  assert.throws(
    () => engine.emit("__RenderPage", [{}, {}]),
    /unsafe parent component ID/,
  );
  assert.equal(brokerCreated, false);
});
