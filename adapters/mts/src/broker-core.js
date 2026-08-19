export const PROTOCOL_VERSION = 1;

const EVENT_OPTIONS = Object.freeze({});
const UNSUPPORTED_ELEMENT_TAGS = new Set([
  "block",
  "component",
  "for",
  "if",
  "list",
  "list-container",
  "list-item",
  "none",
  "page",
  "raw-text",
  "waterfall",
  "wrapper",
]);
const HOST_METHODS = [
  "createElement",
  "createRawText",
  "appendElement",
  "insertElementBefore",
  "removeElement",
  "setAttribute",
  "setId",
  "setClasses",
  "setInlineStyles",
  "addEventListener",
  "removeEventListener",
  "flush",
];

export class BrokerError extends Error {
  constructor(code, message, status) {
    super(message);
    this.name = "BrokerError";
    this.code = code;
    if (status !== undefined) {
      this.status = status;
    }
  }
}

function protocolError(message) {
  throw new BrokerError("E_PROTOCOL", message);
}

function assertPlainObject(value, path) {
  if (value === null || typeof value !== "object" || Array.isArray(value)) {
    protocolError(`${path} must be an object`);
  }
}

function assertExactKeys(value, expected, path) {
  const actual = Object.keys(value).sort();
  const wanted = expected.slice().sort();
  if (actual.length !== wanted.length) {
    protocolError(`${path} has unexpected fields`);
  }
  for (let index = 0; index < actual.length; index += 1) {
    if (actual[index] !== wanted[index]) {
      protocolError(`${path} has unexpected fields`);
    }
  }
}

function assertId(value, path) {
  if (!Number.isSafeInteger(value) || value <= 0) {
    protocolError(`${path} must be a positive safe integer`);
  }
}

function assertString(value, path) {
  if (typeof value !== "string") {
    protocolError(`${path} must be a string`);
  }
}

function nativeError(response) {
  return new BrokerError("E_NATIVE", response.error, response.status);
}

function isElementRef(value) {
  return value !== null && value !== undefined;
}

function cloneNodes(nodes) {
  const result = new Map();
  for (const [id, node] of nodes) {
    result.set(id, {
      kind: node.kind,
      tag: node.tag,
      parent: node.parent,
      children: node.children.slice(),
    });
  }
  return result;
}

function requireNode(nodes, id, path) {
  assertId(id, path);
  const node = nodes.get(id);
  if (node === undefined) {
    protocolError(`${path} refers to unknown node ${id}`);
  }
  return node;
}

function nodeHasListener(listeners, nodeId) {
  for (const listener of listeners.values()) {
    if (listener.nodeId === nodeId) {
      return true;
    }
  }
  return false;
}

function removeChild(parent, childId) {
  const index = parent.children.indexOf(childId);
  if (index === -1) {
    protocolError(`node ${childId} is not a direct child`);
  }
  parent.children.splice(index, 1);
}

function validateMutation(mutation, path, nodes, listeners, rootId) {
  assertPlainObject(mutation, path);
  assertString(mutation.op, `${path}.op`);

  switch (mutation.op) {
    case "create_element": {
      assertExactKeys(mutation, ["op", "node", "tag"], path);
      assertId(mutation.node, `${path}.node`);
      assertString(mutation.tag, `${path}.tag`);
      if (
        mutation.tag.length > 64
        || !/^[a-z][a-z0-9-]*$/.test(mutation.tag)
        || UNSUPPORTED_ELEMENT_TAGS.has(mutation.tag)
      ) {
        protocolError(`${path}.tag is not a supported authored tag`);
      }
      if (nodes.has(mutation.node)) {
        protocolError(`${path}.node ${mutation.node} is already registered`);
      }
      nodes.set(mutation.node, {
        kind: "element",
        tag: mutation.tag,
        parent: null,
        children: [],
      });
      return;
    }

    case "create_text": {
      assertExactKeys(mutation, ["op", "node", "text"], path);
      assertId(mutation.node, `${path}.node`);
      assertString(mutation.text, `${path}.text`);
      if (nodes.has(mutation.node)) {
        protocolError(`${path}.node ${mutation.node} is already registered`);
      }
      nodes.set(mutation.node, {
        kind: "raw-text",
        tag: null,
        parent: null,
        children: [],
      });
      return;
    }

    case "insert_before": {
      assertExactKeys(mutation, ["op", "parent", "child", "reference"], path);
      const parent = requireNode(nodes, mutation.parent, `${path}.parent`);
      const child = requireNode(nodes, mutation.child, `${path}.child`);
      if (mutation.parent === mutation.child) {
        protocolError(`${path} cannot insert a node into itself`);
      }
      if (mutation.child === rootId) {
        protocolError(`${path}.child cannot be the broker root`);
      }
      if (parent.kind === "raw-text") {
        protocolError(`${path}.parent cannot be raw text`);
      }
      if (child.parent !== null) {
        protocolError(`${path}.child ${mutation.child} is already attached`);
      }
      let ancestorId = mutation.parent;
      while (ancestorId !== null) {
        if (ancestorId === mutation.child) {
          protocolError(`${path} would create an ownership cycle`);
        }
        ancestorId = nodes.get(ancestorId).parent;
      }
      if (child.kind === "raw-text" && !(parent.kind === "element" && parent.tag === "text")) {
        protocolError(`${path}.child raw text must be attached to a text element`);
      }
      let index = parent.children.length;
      if (mutation.reference !== null) {
        const reference = requireNode(nodes, mutation.reference, `${path}.reference`);
        if (reference.parent !== mutation.parent) {
          protocolError(`${path}.reference must be a direct child of the parent`);
        }
        index = parent.children.indexOf(mutation.reference);
      }
      parent.children.splice(index, 0, mutation.child);
      child.parent = mutation.parent;
      return;
    }

    case "remove": {
      assertExactKeys(mutation, ["op", "parent", "child"], path);
      const parent = requireNode(nodes, mutation.parent, `${path}.parent`);
      const child = requireNode(nodes, mutation.child, `${path}.child`);
      if (child.parent !== mutation.parent) {
        protocolError(`${path}.child must be a direct child of the parent`);
      }
      removeChild(parent, mutation.child);
      child.parent = null;
      return;
    }

    case "destroy_node": {
      assertExactKeys(mutation, ["op", "node"], path);
      const node = requireNode(nodes, mutation.node, `${path}.node`);
      if (mutation.node === rootId) {
        protocolError(`${path}.node cannot destroy the broker root`);
      }
      if (node.parent !== null || node.children.length !== 0) {
        protocolError(`${path}.node must be detached and childless`);
      }
      if (nodeHasListener(listeners, mutation.node)) {
        protocolError(`${path}.node still has listeners`);
      }
      nodes.delete(mutation.node);
      return;
    }

    case "set_attribute": {
      assertExactKeys(mutation, ["op", "node", "name", "value"], path);
      const node = requireNode(nodes, mutation.node, `${path}.node`);
      assertString(mutation.name, `${path}.name`);
      if (
        mutation.name.length > 128
        || !/^[a-z][a-z0-9_.:-]*$/.test(mutation.name)
        || mutation.name === "constructor"
        || mutation.name === "prototype"
      ) {
        protocolError(`${path}.name is invalid`);
      }
      if (mutation.value !== null && typeof mutation.value !== "string") {
        protocolError(`${path}.value must be a string or null`);
      }
      if (node.kind !== "element") {
        protocolError(`${path}.node must refer to an element`);
      }
      return;
    }

    case "add_event_listener": {
      assertExactKeys(mutation, ["op", "node", "listener", "name"], path);
      const node = requireNode(nodes, mutation.node, `${path}.node`);
      assertId(mutation.listener, `${path}.listener`);
      assertString(mutation.name, `${path}.name`);
      if (node.kind !== "element") {
        protocolError(`${path}.node must refer to an element`);
      }
      if (mutation.name !== "tap") {
        protocolError(`${path}.name must be tap in protocol v1`);
      }
      if (listeners.has(mutation.listener)) {
        protocolError(`${path}.listener ${mutation.listener} is already registered`);
      }
      for (const listener of listeners.values()) {
        if (listener.nodeId === mutation.node && listener.name === mutation.name) {
          protocolError(`${path}.node already has a ${mutation.name} listener`);
        }
      }
      listeners.set(mutation.listener, {
        nodeId: mutation.node,
        name: mutation.name,
      });
      return;
    }

    case "remove_event_listener": {
      assertExactKeys(mutation, ["op", "node", "listener"], path);
      requireNode(nodes, mutation.node, `${path}.node`);
      assertId(mutation.listener, `${path}.listener`);
      const listener = listeners.get(mutation.listener);
      if (listener === undefined) {
        protocolError(`${path}.listener refers to an unknown listener`);
      }
      if (listener.nodeId !== mutation.node) {
        protocolError(`${path} does not match the registered listener`);
      }
      listeners.delete(mutation.listener);
      return;
    }

    case "flush": {
      assertExactKeys(mutation, ["op", "root"], path);
      requireNode(nodes, mutation.root, `${path}.root`);
      if (mutation.root !== rootId) {
        protocolError(`${path}.root must be the broker root ${rootId}`);
      }
      return;
    }

    default:
      protocolError(`${path}.op is not supported by protocol v1`);
  }
}

function decodeResponse(responseJson, currentNodes, currentListeners, rootId, destroy = false) {
  if (typeof responseJson !== "string") {
    protocolError("response must be a JSON string");
  }

  let response;
  try {
    response = JSON.parse(responseJson);
  } catch (error) {
    throw new BrokerError("E_PROTOCOL", `response is not valid JSON: ${error.message}`);
  }

  assertPlainObject(response, "response");
  if (response.ok === true) {
    assertExactKeys(response, ["version", "ok", "operations"], "response");
  } else if (response.ok === false) {
    assertExactKeys(response, ["version", "ok", "status", "error", "operations"], "response");
    if (!Number.isSafeInteger(response.status) || response.status <= 0) {
      protocolError("response.status must be a positive safe integer");
    }
    assertString(response.error, "response.error");
  } else {
    protocolError("response.ok must be a boolean");
  }
  if (response.version !== PROTOCOL_VERSION) {
    protocolError(`response.version must be ${PROTOCOL_VERSION}`);
  }
  if (!Array.isArray(response.operations)) {
    protocolError("response.operations must be an array");
  }
  if (!response.ok && response.operations.length === 0) {
    return {
      response,
      nodes: cloneNodes(currentNodes),
      listeners: new Map(currentListeners),
    };
  }
  if (!response.ok && !destroy) {
    protocolError("only destroy failures may contain cleanup operations");
  }
  if (response.operations.length === 0) {
    protocolError("response.operations must end with one flush operation");
  }

  const nodes = cloneNodes(currentNodes);
  const listeners = new Map(currentListeners);
  let flushCount = 0;
  for (let index = 0; index < response.operations.length; index += 1) {
    const operation = response.operations[index];
    validateMutation(operation, `response.operations[${index}]`, nodes, listeners, rootId);
    if (operation.op === "flush") {
      flushCount += 1;
      if (index !== response.operations.length - 1) {
        protocolError("flush must be the final operation");
      }
    }
  }
  if (flushCount !== 1) {
    protocolError("response.operations must contain exactly one flush operation");
  }
  return { response, nodes, listeners };
}

export function createBroker(options) {
  assertPlainObject(options, "broker options");
  const {
    host,
    nativeModule,
    root,
    rootId = 1,
    suppressMountFlush = false,
  } = options;
  assertPlainObject(host, "broker options.host");
  for (const method of HOST_METHODS) {
    if (typeof host[method] !== "function") {
      throw new BrokerError("E_HOST", `host.${method} must be a function`);
    }
  }
  if (nativeModule === null || typeof nativeModule !== "object" || typeof nativeModule.invoke !== "function") {
    throw new BrokerError("E_HOST", "nativeModule.invoke must be a function");
  }
  if (!isElementRef(root)) {
    throw new BrokerError("E_HOST", "root must be an opaque ElementRef value");
  }
  assertId(rootId, "broker options.rootId");
  if (typeof suppressMountFlush !== "boolean") {
    throw new BrokerError("E_HOST", "broker options.suppressMountFlush must be a boolean");
  }

  const refs = new Map([[rootId, root]]);
  const nodes = new Map([
    [rootId, { kind: "root", tag: "page", parent: null, children: [] }],
  ]);
  const listeners = new Map();
  let busy = false;
  let destroyed = false;
  let poisoned = false;
  let mounted = false;

  function ensureUsable() {
    if (destroyed) {
      throw new BrokerError("E_DESTROYED", "broker has been destroyed");
    }
    if (poisoned) {
      throw new BrokerError("E_HOST", "broker is unusable after a host mutation failed");
    }
  }

  function runExclusive(operation, callback) {
    ensureUsable();
    if (busy) {
      throw new BrokerError("E_REENTRANT", `${operation} cannot run while another broker operation is active`);
    }
    busy = true;
    try {
      return callback();
    } finally {
      busy = false;
    }
  }

  function getRef(id) {
    const ref = refs.get(id);
    if (ref === undefined) {
      throw new BrokerError("E_HOST", `validated node ${id} has no ElementRef`);
    }
    return ref;
  }

  function dispatchListener(listenerId, eventName) {
    return runExclusive("event dispatch", () => {
      const listener = listeners.get(listenerId);
      if (listener === undefined || listener.name !== eventName) {
        throw new BrokerError("E_LISTENER", `listener ${listenerId} is no longer registered`);
      }
      const responseJson = nativeModule.invoke("dispatch", String(listenerId), eventName);
      const decoded = decodeResponse(responseJson, nodes, listeners, rootId);
      if (!decoded.response.ok) {
        throw nativeError(decoded.response);
      }
      applyValidated(decoded.response.operations);
    });
  }

  function applyMutation(mutation) {
    switch (mutation.op) {
      case "create_element": {
        const ref = host.createElement(mutation.tag);
        if (!isElementRef(ref)) {
          throw new BrokerError("E_HOST", "host.createElement did not return an opaque ElementRef value");
        }
        refs.set(mutation.node, ref);
        nodes.set(mutation.node, {
          kind: "element",
          tag: mutation.tag,
          parent: null,
          children: [],
        });
        return;
      }

      case "create_text": {
        const ref = host.createRawText(mutation.text);
        if (!isElementRef(ref)) {
          throw new BrokerError("E_HOST", "host.createRawText did not return an opaque ElementRef value");
        }
        refs.set(mutation.node, ref);
        nodes.set(mutation.node, {
          kind: "raw-text",
          tag: null,
          parent: null,
          children: [],
        });
        return;
      }

      case "insert_before": {
        const parentRef = getRef(mutation.parent);
        const childRef = getRef(mutation.child);
        if (mutation.reference === null) {
          host.appendElement(parentRef, childRef);
        } else {
          host.insertElementBefore(parentRef, childRef, getRef(mutation.reference));
        }
        const parent = nodes.get(mutation.parent);
        const child = nodes.get(mutation.child);
        const index = mutation.reference === null
          ? parent.children.length
          : parent.children.indexOf(mutation.reference);
        parent.children.splice(index, 0, mutation.child);
        child.parent = mutation.parent;
        return;
      }

      case "remove": {
        host.removeElement(getRef(mutation.parent), getRef(mutation.child));
        const parent = nodes.get(mutation.parent);
        const child = nodes.get(mutation.child);
        removeChild(parent, mutation.child);
        child.parent = null;
        return;
      }

      case "destroy_node":
        refs.delete(mutation.node);
        nodes.delete(mutation.node);
        return;

      case "set_attribute": {
        const ref = getRef(mutation.node);
        if (mutation.name === "id") {
          host.setId(ref, mutation.value === null ? "" : mutation.value);
        } else if (mutation.name === "class") {
          host.setClasses(ref, mutation.value === null ? "" : mutation.value);
        } else if (mutation.name === "style") {
          host.setInlineStyles(ref, mutation.value === null ? "" : mutation.value);
        } else {
          host.setAttribute(ref, mutation.name, mutation.value === null ? undefined : mutation.value);
        }
        return;
      }

      case "add_event_listener": {
        const callback = () => dispatchListener(mutation.listener, mutation.name);
        host.addEventListener(getRef(mutation.node), mutation.name, callback, EVENT_OPTIONS);
        listeners.set(mutation.listener, {
          nodeId: mutation.node,
          name: mutation.name,
          callback,
          options: EVENT_OPTIONS,
        });
        return;
      }

      case "remove_event_listener": {
        const listener = listeners.get(mutation.listener);
        host.removeEventListener(
          getRef(mutation.node),
          listener.name,
          listener.callback,
          listener.options,
        );
        listeners.delete(mutation.listener);
        return;
      }

      case "flush":
        host.flush(getRef(mutation.root));
        return;
    }
  }

  function applyValidated(operations, skipFlush = false) {
    try {
      for (const operation of operations) {
        if (skipFlush && operation.op === "flush") {
          continue;
        }
        applyMutation(operation);
      }
    } catch (error) {
      poisoned = true;
      throw error;
    }
  }

  function applyBatch(responseJson) {
    return runExclusive("batch application", () => {
      const decoded = decodeResponse(responseJson, nodes, listeners, rootId);
      if (!decoded.response.ok) {
        throw nativeError(decoded.response);
      }
      applyValidated(decoded.response.operations);
    });
  }

  function mount() {
    return runExclusive("mount", () => {
      if (mounted) {
        throw new BrokerError("E_MOUNTED", "broker mount may only run once");
      }
      const responseJson = nativeModule.invoke("mount", String(rootId));
      const decoded = decodeResponse(responseJson, nodes, listeners, rootId);
      if (!decoded.response.ok) {
        throw nativeError(decoded.response);
      }
      applyValidated(decoded.response.operations, suppressMountFlush);
      mounted = true;
    });
  }

  function recordFailure(current, error) {
    return current === null ? error : current;
  }

  function detachRemainingNodes() {
    const attached = [];
    for (const [id, node] of nodes) {
      if (id !== rootId && node.parent !== null) {
        let depth = 0;
        let cursor = node;
        while (cursor.parent !== null) {
          depth += 1;
          cursor = nodes.get(cursor.parent);
        }
        attached.push({ id, depth });
      }
    }
    attached.sort((left, right) => right.depth - left.depth);

    let failure = null;
    for (const { id } of attached) {
      const node = nodes.get(id);
      if (node === undefined || node.parent === null) {
        continue;
      }
      const parentId = node.parent;
      try {
        host.removeElement(getRef(parentId), getRef(id));
        const parent = nodes.get(parentId);
        const index = parent.children.indexOf(id);
        if (index !== -1) {
          parent.children.splice(index, 1);
        }
        node.parent = null;
      } catch (error) {
        failure = recordFailure(failure, error);
      }
    }
    return failure;
  }

  function destroy() {
    if (destroyed) {
      return;
    }
    if (busy) {
      throw new BrokerError("E_REENTRANT", "destroy cannot run while another broker operation is active");
    }

    busy = true;
    let failure = null;
    try {
      try {
        const responseJson = nativeModule.invoke("destroySession");
        const decoded = decodeResponse(responseJson, nodes, listeners, rootId, true);
        if (decoded.response.ok && (decoded.nodes.size !== 1 || decoded.listeners.size !== 0)) {
          protocolError("successful destroy response must release every non-root node and listener");
        }
        if (!decoded.response.ok) {
          failure = recordFailure(failure, nativeError(decoded.response));
        }
        if (decoded.response.operations.length !== 0) {
          applyValidated(decoded.response.operations, true);
        }
      } catch (error) {
        failure = recordFailure(failure, error);
      }

      for (const listener of listeners.values()) {
        try {
          host.removeEventListener(
            getRef(listener.nodeId),
            listener.name,
            listener.callback,
            listener.options,
          );
        } catch (error) {
          failure = recordFailure(failure, error);
        }
      }
      listeners.clear();

      failure = recordFailure(failure, detachRemainingNodes());
      try {
        host.flush(root);
      } catch (error) {
        failure = recordFailure(failure, error);
      }
    } finally {
      listeners.clear();
      refs.clear();
      nodes.clear();
      destroyed = true;
      poisoned = false;
      busy = false;
    }

    if (failure !== null) {
      throw failure;
    }
  }

  return Object.freeze({
    applyBatch,
    mount,
    destroy,
  });
}
