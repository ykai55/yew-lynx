import assert from "node:assert/strict";

export function batch(operations, root = 1) {
  return JSON.stringify({
    version: 1,
    ok: true,
    operations: [...operations, { op: "flush", root }],
  });
}

export function failure(status, error, operations = [], root = 1) {
  return JSON.stringify({
    version: 1,
    ok: false,
    status,
    error,
    operations: operations.length === 0
      ? []
      : [...operations, { op: "flush", root }],
  });
}

export function createFakeNativeModule(initialHandlers = {}) {
  const handlers = new Map(Object.entries(initialHandlers));
  const calls = [];
  return {
    calls,
    invoke(method, ...args) {
      calls.push([method, ...args]);
      const handler = handlers.get(method);
      if (handler === undefined) {
        throw new Error(`unexpected native invocation: ${method}`);
      }
      return handler(...args);
    },
    setHandler(method, handler) {
      handlers.set(method, handler);
    },
  };
}

export function createStrictFiberHost() {
  let nextRefId = 1;
  const refs = [];
  const calls = [];
  const flushes = [];

  function allocate(kind, tag, text = null) {
    const ref = {
      refId: nextRefId,
      kind,
      tag,
      text,
      parent: null,
      children: [],
      attributes: new Map(),
      idValue: null,
      classes: undefined,
      inlineStyles: null,
      listeners: new Map(),
    };
    nextRefId += 1;
    refs.push(ref);
    return ref;
  }

  const root = allocate("root", "page");

  function requireRef(ref) {
    assert.ok(refs.includes(ref), "host received an unknown ElementRef");
    return ref;
  }

  function requireParent(parent) {
    requireRef(parent);
    assert.notEqual(parent.kind, "raw-text", "raw text cannot own children");
  }

  function requireAttachable(parent, child) {
    requireParent(parent);
    requireRef(child);
    assert.equal(child.parent, null, "child must be detached before insertion");
    if (child.kind === "raw-text") {
      assert.equal(parent.tag, "text", "raw text must be inserted under text");
    }
  }

  function snapshot(ref) {
    return {
      kind: ref.kind,
      tag: ref.tag,
      text: ref.text,
      id: ref.idValue,
      classes: ref.classes,
      style: ref.inlineStyles,
      attributes: Object.fromEntries(ref.attributes),
      children: ref.children.map(snapshot),
    };
  }

  const host = {
    createElement(tag) {
      const kind = tag === "view" ? "view" : tag === "text" ? "text" : "element";
      calls.push(["createElement", tag]);
      return allocate(kind, tag);
    },

    createRawText(text) {
      calls.push(["createRawText", text]);
      return allocate("raw-text", null, text);
    },

    appendElement(parent, child) {
      requireAttachable(parent, child);
      calls.push(["appendElement", parent.refId, child.refId]);
      parent.children.push(child);
      child.parent = parent;
    },

    insertElementBefore(parent, child, reference) {
      requireAttachable(parent, child);
      requireRef(reference);
      assert.equal(reference.parent, parent, "reference must be a direct child");
      calls.push(["insertElementBefore", parent.refId, child.refId, reference.refId]);
      parent.children.splice(parent.children.indexOf(reference), 0, child);
      child.parent = parent;
    },

    removeElement(parent, child) {
      requireParent(parent);
      requireRef(child);
      assert.equal(child.parent, parent, "remove requires a direct child");
      calls.push(["removeElement", parent.refId, child.refId]);
      parent.children.splice(parent.children.indexOf(child), 1);
      child.parent = null;
    },

    setAttribute(element, name, value) {
      requireRef(element);
      assert.notEqual(element.kind, "raw-text");
      calls.push(["setAttribute", element.refId, name, value]);
      if (value === undefined) {
        element.attributes.delete(name);
      } else {
        element.attributes.set(name, value);
      }
    },

    setId(element, value) {
      requireRef(element);
      calls.push(["setId", element.refId, value]);
      element.idValue = value;
    },

    setClasses(element, value) {
      requireRef(element);
      calls.push(["setClasses", element.refId, value]);
      element.classes = value;
    },

    setInlineStyles(element, value) {
      requireRef(element);
      calls.push(["setInlineStyles", element.refId, value]);
      element.inlineStyles = value;
    },

    addEventListener(element, name, callback, options) {
      requireRef(element);
      assert.equal(typeof callback, "function");
      const callbacks = element.listeners.get(name) || new Map();
      assert.equal(callbacks.has(callback), false, "callback already registered");
      callbacks.set(callback, options);
      element.listeners.set(name, callbacks);
      calls.push(["addEventListener", element.refId, name]);
    },

    removeEventListener(element, name, callback, options) {
      requireRef(element);
      const callbacks = element.listeners.get(name);
      assert.ok(callbacks && callbacks.has(callback), "callback is not registered");
      assert.equal(callbacks.get(callback), options, "remove must reuse listener options");
      callbacks.delete(callback);
      if (callbacks.size === 0) {
        element.listeners.delete(name);
      }
      calls.push(["removeEventListener", element.refId, name]);
    },

    flush(element) {
      assert.equal(element, root, "only the page root may be flushed");
      calls.push(["flush", root.refId]);
      flushes.push(snapshot(root));
    },
  };

  function findById(id) {
    return refs.find((ref) => ref.idValue === id);
  }

  function emitTapById(id) {
    const ref = findById(id);
    assert.ok(ref, `element with id ${id} not found`);
    const callbacks = ref.listeners.get("tap");
    assert.ok(callbacks && callbacks.size > 0, `element ${id} has no tap listener`);
    for (const callback of Array.from(callbacks.keys())) {
      callback({ type: "tap" });
    }
  }

  function listenerCount() {
    let count = 0;
    for (const ref of refs) {
      for (const callbacks of ref.listeners.values()) {
        count += callbacks.size;
      }
    }
    return count;
  }

  return {
    host,
    root,
    refs,
    calls,
    flushes,
    snapshot: () => snapshot(root),
    findById,
    emitTapById,
    listenerCount,
  };
}
