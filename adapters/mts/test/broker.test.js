import assert from "node:assert/strict";
import test from "node:test";

import { BrokerError, createBroker } from "../src/broker-core.js";
import {
  batch,
  createFakeNativeModule,
  createStrictFiberHost,
  failure,
} from "./fakes.js";

function makeBroker(fake, nativeModule) {
  return createBroker({
    host: fake.host,
    nativeModule,
    root: fake.root,
    rootId: 1,
  });
}

test("numeric native ElementRefs remain opaque to the broker", () => {
  const calls = [];
  const host = {
    createElement() {
      return 102;
    },
    createRawText() {
      return 103;
    },
    appendElement(parent, child) {
      calls.push(["appendElement", parent, child]);
    },
    insertElementBefore() {},
    removeElement() {},
    setAttribute() {},
    setId() {},
    setClasses() {},
    setInlineStyles() {},
    addEventListener() {},
    removeEventListener() {},
    flush(root) {
      calls.push(["flush", root]);
    },
  };
  const nativeModule = createFakeNativeModule({
    mount() {
      return batch([
        { op: "create_element", node: 2, tag: "text" },
        { op: "create_text", node: 3, text: "Count: 0" },
        { op: "insert_before", parent: 1, child: 2, reference: null },
        { op: "insert_before", parent: 2, child: 3, reference: null },
      ]);
    },
  });

  createBroker({ host, nativeModule, root: 101 }).mount();

  assert.deepEqual(calls, [
    ["appendElement", 101, 102],
    ["appendElement", 102, 103],
    ["flush", 101],
  ]);
});

test("mount applies protocol v1 through a strict Fiber host and flushes once", () => {
  const fake = createStrictFiberHost();
  const nativeModule = createFakeNativeModule({
    mount(rootId) {
      assert.equal(rootId, "1");
      return batch([
        { op: "create_element", node: 2, tag: "view" },
        { op: "create_element", node: 3, tag: "text" },
        { op: "create_element", node: 4, tag: "image" },
        { op: "create_text", node: 5, text: "Count: 0" },
        { op: "set_attribute", node: 2, name: "id", value: "counter" },
        { op: "set_attribute", node: 2, name: "class", value: "card active" },
        { op: "set_attribute", node: 2, name: "style", value: "padding: 8px" },
        { op: "set_attribute", node: 2, name: "data-state", value: "ready" },
        { op: "insert_before", parent: 1, child: 2, reference: null },
        { op: "insert_before", parent: 2, child: 3, reference: null },
        { op: "insert_before", parent: 3, child: 5, reference: null },
        { op: "insert_before", parent: 2, child: 4, reference: 3 },
        { op: "add_event_listener", node: 2, listener: 10, name: "tap" },
      ]);
    },
  });

  makeBroker(fake, nativeModule).mount();

  assert.equal(fake.flushes.length, 1);
  assert.deepEqual(fake.calls.filter(([name]) => name === "createElement"), [
    ["createElement", "view"],
    ["createElement", "text"],
    ["createElement", "image"],
  ]);
  assert.equal(fake.calls.filter(([name]) => name === "appendElement").length, 3);
  assert.equal(fake.calls.filter(([name]) => name === "insertElementBefore").length, 1);
  assert.deepEqual(fake.snapshot(), {
    kind: "root",
    tag: "page",
    text: null,
    id: null,
    classes: undefined,
    style: null,
    attributes: {},
    children: [
      {
        kind: "view",
        tag: "view",
        text: null,
        id: "counter",
        classes: "card active",
        style: "padding: 8px",
        attributes: { "data-state": "ready" },
        children: [
          {
            kind: "element",
            tag: "image",
            text: null,
            id: null,
            classes: undefined,
            style: null,
            attributes: {},
            children: [],
          },
          {
            kind: "text",
            tag: "text",
            text: null,
            id: null,
            classes: undefined,
            style: null,
            attributes: {},
            children: [
              {
                kind: "raw-text",
                tag: null,
                text: "Count: 0",
                id: null,
                classes: undefined,
                style: null,
                attributes: {},
                children: [],
              },
            ],
          },
        ],
      },
    ],
  });
});

test("the complete batch is validated before any host mutation", () => {
  const fake = createStrictFiberHost();
  const nativeModule = createFakeNativeModule();
  const broker = makeBroker(fake, nativeModule);

  assert.throws(
    () => broker.applyBatch(batch([
      { op: "create_element", node: 2, tag: "view" },
      { op: "set_attribute", node: 99, name: "id", value: "missing" },
    ])),
    (error) => error instanceof BrokerError && error.code === "E_PROTOCOL",
  );
  assert.deepEqual(fake.calls, []);
  assert.equal(fake.flushes.length, 0);

  broker.applyBatch(batch([
    { op: "create_element", node: 2, tag: "view" },
    { op: "set_attribute", node: 2, name: "id", value: "valid" },
    { op: "insert_before", parent: 1, child: 2, reference: null },
  ]));
  assert.equal(fake.flushes.length, 1);
  assert.ok(fake.findById("valid"));
});

test("protocol v1 rejects malformed ownership and unsupported surface", () => {
  const cases = [
    "not json",
    JSON.stringify({ version: 1, operations: [{ op: "flush", root: 1 }] }),
    JSON.stringify({ version: 2, ok: true, operations: [{ op: "flush", root: 1 }] }),
    JSON.stringify({ version: 1, ok: true, operations: [{ op: "flush", root: 1 }], extra: true }),
    JSON.stringify({ version: 1, ok: true, status: 1, operations: [{ op: "flush", root: 1 }] }),
    JSON.stringify({ version: 1, ok: false, status: 1, error: "failed", operations: [], extra: true }),
    JSON.stringify({ version: 1, ok: false, status: 1, operations: [] }),
    JSON.stringify({ version: 1, ok: false, status: 1, error: 1, operations: [] }),
    JSON.stringify({ version: 1, ok: false, status: 1.5, error: "failed", operations: [] }),
    JSON.stringify({ version: 1, ok: true, operations: [] }),
    JSON.stringify({ version: 1, ok: true, operations: [{ op: "flush", root: 1 }, { op: "flush", root: 1 }] }),
    batch([{ op: "create_element", node: 2, tag: "Page" }]),
    batch([{ op: "create_element", node: 2, tag: "list" }]),
    batch([{ op: "create_element", node: 2, tag: "list-container" }]),
    batch([{ op: "create_element", node: 2, tag: "waterfall" }]),
    batch([{ op: "create_element", node: Number.MAX_SAFE_INTEGER + 1, tag: "view" }]),
    batch([{ op: "create_element", node: 2, tag: "view", extra: true }]),
    batch([{ op: "create_element", node: 2, tag: `x-${"a".repeat(64)}` }]),
    batch([{ op: "create_element", node: 2, tag: "view" }, { op: "create_element", node: 2, tag: "text" }]),
    batch([{ op: "create_text", node: 2, text: "raw" }, { op: "insert_before", parent: 1, child: 2, reference: null }]),
    batch([{ op: "create_element", node: 2, tag: "view" }, { op: "add_event_listener", node: 2, listener: 1, name: "click" }]),
    batch([{ op: "create_element", node: 2, tag: "view" }, { op: "add_event_listener", node: 2, listener: Number.MAX_SAFE_INTEGER + 1, name: "tap" }]),
    batch([{ op: "create_element", node: 2, tag: "view" }, { op: "set_attribute", node: 2, name: "Constructor", value: "bad" }]),
    batch([{ op: "create_element", node: 2, tag: "view" }, { op: "set_attribute", node: 2, name: "a".repeat(129), value: "bad" }]),
    failure(6, "bad listener", [{ op: "flush", root: 1 }]),
  ];

  for (const invalid of cases) {
    const fake = createStrictFiberHost();
    const broker = makeBroker(fake, createFakeNativeModule());
    assert.throws(
      () => broker.applyBatch(invalid),
      (error) => error instanceof BrokerError && error.code === "E_PROTOCOL",
    );
    assert.deepEqual(fake.calls, []);
  }
});

test("insert validation rejects self insertion and detached-subtree cycles", () => {
  const invalidBatches = [
    batch([
      { op: "create_element", node: 2, tag: "view" },
      { op: "insert_before", parent: 2, child: 2, reference: null },
    ]),
    batch([
      { op: "create_element", node: 2, tag: "view" },
      { op: "create_element", node: 3, tag: "view" },
      { op: "insert_before", parent: 2, child: 3, reference: null },
      { op: "insert_before", parent: 3, child: 2, reference: null },
    ]),
  ];

  for (const response of invalidBatches) {
    const fake = createStrictFiberHost();
    const broker = makeBroker(fake, createFakeNativeModule());
    assert.throws(
      () => broker.applyBatch(response),
      (error) => error instanceof BrokerError && error.code === "E_PROTOCOL",
    );
    assert.deepEqual(fake.calls, []);
  }
});

test("listener validation rejects duplicate IDs and duplicate node-event registrations", () => {
  const invalidBatches = [
    batch([
      { op: "create_element", node: 2, tag: "view" },
      { op: "add_event_listener", node: 2, listener: 10, name: "tap" },
      { op: "add_event_listener", node: 2, listener: 10, name: "tap" },
    ]),
    batch([
      { op: "create_element", node: 2, tag: "view" },
      { op: "add_event_listener", node: 2, listener: 10, name: "tap" },
      { op: "add_event_listener", node: 2, listener: 11, name: "tap" },
    ]),
  ];

  for (const response of invalidBatches) {
    const fake = createStrictFiberHost();
    const broker = makeBroker(fake, createFakeNativeModule());
    assert.throws(
      () => broker.applyBatch(response),
      (error) => error instanceof BrokerError && error.code === "E_PROTOCOL",
    );
    assert.deepEqual(fake.calls, []);
  }
});

test("native failures retain their status and cannot carry ordinary mutations", () => {
  const fake = createStrictFiberHost();
  const broker = makeBroker(fake, createFakeNativeModule());

  assert.throws(
    () => broker.applyBatch(failure(6, "invalid listener")),
    (error) => error instanceof BrokerError
      && error.code === "E_NATIVE"
      && error.status === 6
      && error.message === "invalid listener",
  );
  assert.deepEqual(fake.calls, []);

  assert.throws(
    () => broker.applyBatch(failure(6, "invalid listener", [
      { op: "set_attribute", node: 1, name: "data-state", value: "bad" },
    ])),
    (error) => error instanceof BrokerError && error.code === "E_PROTOCOL",
  );
  assert.deepEqual(fake.calls, []);
});

test("attribute clears use the exact public Fiber API sentinel values", () => {
  const fake = createStrictFiberHost();
  const broker = makeBroker(fake, createFakeNativeModule());
  broker.applyBatch(batch([
    { op: "create_element", node: 2, tag: "view" },
    { op: "set_attribute", node: 2, name: "id", value: "target" },
    { op: "set_attribute", node: 2, name: "class", value: "active" },
    { op: "set_attribute", node: 2, name: "style", value: "opacity: 1" },
    { op: "set_attribute", node: 2, name: "data-state", value: "ready" },
    { op: "insert_before", parent: 1, child: 2, reference: null },
  ]));
  const element = fake.findById("target");

  broker.applyBatch(batch([
    { op: "set_attribute", node: 2, name: "class", value: null },
    { op: "set_attribute", node: 2, name: "style", value: null },
    { op: "set_attribute", node: 2, name: "data-state", value: null },
    { op: "set_attribute", node: 2, name: "id", value: null },
  ]));

  assert.equal(element.classes, "");
  assert.equal(element.inlineStyles, "");
  assert.equal(element.attributes.has("data-state"), false);
  assert.equal(element.idValue, "");
  assert.deepEqual(fake.calls.slice(-5), [
    ["setClasses", element.refId, ""],
    ["setInlineStyles", element.refId, ""],
    ["setAttribute", element.refId, "data-state", undefined],
    ["setId", element.refId, ""],
    ["flush", fake.root.refId],
  ]);
});

test("mount and dispatch pass decimal String IDs and mount flush can be suppressed", () => {
  const fake = createStrictFiberHost();
  const nativeModule = createFakeNativeModule({
    mount(rootId) {
      assert.equal(rootId, "1");
      assert.equal(typeof rootId, "string");
      return batch([
        { op: "create_element", node: 2, tag: "view" },
        { op: "set_attribute", node: 2, name: "id", value: "button" },
        { op: "insert_before", parent: 1, child: 2, reference: null },
        { op: "add_event_listener", node: 2, listener: 10, name: "tap" },
      ]);
    },
    dispatch(listenerId, eventName) {
      assert.equal(listenerId, "10");
      assert.equal(typeof listenerId, "string");
      assert.equal(eventName, "tap");
      return batch([]);
    },
  });
  const broker = createBroker({
    host: fake.host,
    nativeModule,
    root: fake.root,
    rootId: 1,
    suppressMountFlush: true,
  });

  broker.mount();
  assert.equal(fake.flushes.length, 0);
  fake.emitTapById("button");
  assert.equal(fake.flushes.length, 1);
});

test("tap dispatch is synchronous and listener removal uses the exact callback", () => {
  const fake = createStrictFiberHost();
  const nativeModule = createFakeNativeModule({
    dispatch(listenerId, name) {
      assert.equal(listenerId, "10");
      assert.equal(name, "tap");
      return batch([
        { op: "set_attribute", node: 2, name: "data-count", value: "1" },
      ]);
    },
  });
  const broker = makeBroker(fake, nativeModule);
  broker.applyBatch(batch([
    { op: "create_element", node: 2, tag: "view" },
    { op: "set_attribute", node: 2, name: "id", value: "button" },
    { op: "insert_before", parent: 1, child: 2, reference: null },
    { op: "add_event_listener", node: 2, listener: 10, name: "tap" },
  ]));

  fake.emitTapById("button");
  assert.deepEqual(nativeModule.calls.at(-1), ["dispatch", "10", "tap"]);
  assert.equal(fake.findById("button").attributes.get("data-count"), "1");
  assert.equal(fake.flushes.length, 2);

  broker.applyBatch(batch([
    { op: "remove_event_listener", node: 2, listener: 10 },
  ]));
  assert.equal(fake.listenerCount(), 0);
  assert.throws(() => fake.emitTapById("button"), /has no tap listener/);
});

test("invalid dispatch batches and reentrant callbacks cannot partially mutate", () => {
  const fake = createStrictFiberHost();
  const nativeModule = createFakeNativeModule();
  const broker = makeBroker(fake, nativeModule);
  broker.applyBatch(batch([
    { op: "create_element", node: 2, tag: "view" },
    { op: "set_attribute", node: 2, name: "id", value: "button" },
    { op: "insert_before", parent: 1, child: 2, reference: null },
    { op: "add_event_listener", node: 2, listener: 10, name: "tap" },
  ]));

  nativeModule.setHandler("dispatch", () => batch([
    { op: "set_attribute", node: 2, name: "data-state", value: "bad" },
    { op: "remove", parent: 1, child: 99 },
  ]));
  const callsBeforeInvalid = fake.calls.length;
  assert.throws(
    () => fake.emitTapById("button"),
    (error) => error instanceof BrokerError && error.code === "E_PROTOCOL",
  );
  assert.equal(fake.calls.length, callsBeforeInvalid);
  assert.equal(fake.findById("button").attributes.has("data-state"), false);

  nativeModule.setHandler("dispatch", () => {
    fake.emitTapById("button");
    return batch([]);
  });
  assert.throws(
    () => fake.emitTapById("button"),
    (error) => error instanceof BrokerError && error.code === "E_REENTRANT",
  );

  nativeModule.setHandler("dispatch", () => batch([
    { op: "set_attribute", node: 2, name: "data-state", value: "recovered" },
  ]));
  fake.emitTapById("button");
  assert.equal(fake.findById("button").attributes.get("data-state"), "recovered");
});

test("destroy applies cleanup, flushes once, and invalidates all IDs", () => {
  const fake = createStrictFiberHost();
  const nativeModule = createFakeNativeModule({
    destroySession: () => batch([
      { op: "remove_event_listener", node: 2, listener: 10 },
      { op: "remove", parent: 3, child: 4 },
      { op: "destroy_node", node: 4 },
      { op: "remove", parent: 2, child: 3 },
      { op: "destroy_node", node: 3 },
      { op: "remove", parent: 1, child: 2 },
      { op: "destroy_node", node: 2 },
    ]),
  });
  const broker = makeBroker(fake, nativeModule);
  broker.applyBatch(batch([
    { op: "create_element", node: 2, tag: "view" },
    { op: "create_element", node: 3, tag: "text" },
    { op: "create_text", node: 4, text: "bye" },
    { op: "set_attribute", node: 2, name: "id", value: "button" },
    { op: "insert_before", parent: 1, child: 2, reference: null },
    { op: "insert_before", parent: 2, child: 3, reference: null },
    { op: "insert_before", parent: 3, child: 4, reference: null },
    { op: "add_event_listener", node: 2, listener: 10, name: "tap" },
  ]));
  const flushesBeforeDestroy = fake.flushes.length;

  broker.destroy();

  assert.deepEqual(nativeModule.calls.at(-1), ["destroySession"]);
  assert.equal(fake.flushes.length, flushesBeforeDestroy + 1);
  assert.equal(fake.root.children.length, 0);
  assert.equal(fake.listenerCount(), 0);
  assert.throws(
    () => broker.applyBatch(batch([])),
    (error) => error instanceof BrokerError && error.code === "E_DESTROYED",
  );

  const callsAfterDestroy = nativeModule.calls.length;
  broker.destroy();
  assert.equal(nativeModule.calls.length, callsAfterDestroy);
});

test("destroy clears host state even when the native cleanup batch is invalid", () => {
  const fake = createStrictFiberHost();
  const nativeModule = createFakeNativeModule({
    destroySession: () => batch([
      { op: "set_attribute", node: 2, name: "data-state", value: "must-not-apply" },
      { op: "destroy_node", node: 99 },
    ]),
  });
  const broker = makeBroker(fake, nativeModule);
  broker.applyBatch(batch([
    { op: "create_element", node: 2, tag: "view" },
    { op: "set_attribute", node: 2, name: "id", value: "button" },
    { op: "insert_before", parent: 1, child: 2, reference: null },
    { op: "add_event_listener", node: 2, listener: 10, name: "tap" },
  ]));
  const flushesBeforeDestroy = fake.flushes.length;

  assert.throws(
    () => broker.destroy(),
    (error) => error instanceof BrokerError && error.code === "E_PROTOCOL",
  );

  assert.equal(fake.findById("button").attributes.has("data-state"), false);
  assert.equal(fake.root.children.length, 0);
  assert.equal(fake.listenerCount(), 0);
  assert.equal(fake.flushes.length, flushesBeforeDestroy + 1);
  assert.throws(
    () => broker.applyBatch(batch([])),
    (error) => error instanceof BrokerError && error.code === "E_DESTROYED",
  );
});

test("destroy applies valid cleanup from a failed native response before surfacing status", () => {
  const fake = createStrictFiberHost();
  const nativeModule = createFakeNativeModule({
    destroySession: () => failure(9, "teardown panic", [
      { op: "remove_event_listener", node: 2, listener: 10 },
      { op: "remove", parent: 1, child: 2 },
      { op: "destroy_node", node: 2 },
    ]),
  });
  const broker = makeBroker(fake, nativeModule);
  broker.applyBatch(batch([
    { op: "create_element", node: 2, tag: "view" },
    { op: "set_attribute", node: 2, name: "id", value: "button" },
    { op: "insert_before", parent: 1, child: 2, reference: null },
    { op: "add_event_listener", node: 2, listener: 10, name: "tap" },
  ]));
  const flushesBeforeDestroy = fake.flushes.length;

  assert.throws(
    () => broker.destroy(),
    (error) => error instanceof BrokerError
      && error.code === "E_NATIVE"
      && error.status === 9
      && error.message === "teardown panic",
  );

  assert.equal(fake.root.children.length, 0);
  assert.equal(fake.listenerCount(), 0);
  assert.equal(fake.flushes.length, flushesBeforeDestroy + 1);
  assert.deepEqual(fake.calls.slice(-3), [
    ["removeEventListener", 2, "tap"],
    ["removeElement", 1, 2],
    ["flush", 1],
  ]);
});
