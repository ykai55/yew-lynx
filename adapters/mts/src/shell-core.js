import { createBroker } from "./broker-core.js";

const ROOT_ID = 1;

function eventArguments(event) {
  return event !== null && typeof event === "object" && Array.isArray(event.data)
    ? event.data
    : [];
}

function eventOptions(event) {
  const options = eventArguments(event)[1];
  return options !== null && typeof options === "object" && !Array.isArray(options)
    ? options
    : {};
}

function hasNonEmptyCache(cacheData) {
  if (cacheData === null || cacheData === undefined) {
    return false;
  }
  if (Array.isArray(cacheData) || typeof cacheData === "string") {
    return cacheData.length !== 0;
  }
  return true;
}

export function installLynxElementBridgeShell({
  lynxApi,
  createPage,
  getElementUniqueId,
  createHost,
  brokerFactory = createBroker,
}) {
  let activeBroker = null;

  function destroyActiveBroker() {
    if (activeBroker === null) {
      return;
    }
    const broker = activeBroker;
    activeBroker = null;
    broker.destroy();
  }

  function destroyActiveBrokerForLifecycle() {
    try {
      destroyActiveBroker();
    } catch (error) {
      lynxApi.reportError(error);
    }
  }

  function mount(renderOptions, suppressMountFlush) {
    if (activeBroker !== null) {
      throw new Error("LynxElementBridge broker is already mounted");
    }
    if (renderOptions.initPage !== null && renderOptions.initPage !== undefined) {
      throw new Error("LynxElementBridge does not support cached or SSR initPage roots");
    }
    if (hasNonEmptyCache(renderOptions.cacheData)) {
      throw new Error("LynxElementBridge does not support cached render data");
    }
    if (hasNonEmptyCache(renderOptions.hydrateMap)) {
      throw new Error("LynxElementBridge does not support SSR hydration");
    }

    const page = createPage("0", 0);
    const parentComponentId = getElementUniqueId(page);
    if (!Number.isSafeInteger(parentComponentId) || parentComponentId <= 0) {
      throw new Error("LynxElementBridge page has an unsafe parent component ID");
    }
    const candidate = brokerFactory({
      host: createHost(parentComponentId),
      nativeModule: lynxApi.module("LynxElementBridge"),
      root: page,
      rootId: ROOT_ID,
      suppressMountFlush,
    });
    try {
      candidate.mount();
      activeBroker = candidate;
    } catch (error) {
      try {
        candidate.destroy();
      } catch {
        // Preserve the mount failure; destroy is only best-effort here.
      }
      throw error;
    }
  }

  const engine = lynxApi.getEngine();
  engine.addEventListener("__RenderPage", (event) => {
    mount(eventOptions(event), true);
  });
  engine.addEventListener("__UpdatePage", (event) => {
    const options = eventOptions(event);
    const reload = options.reloadTemplate === true || options.reloadFromJS === true;
    if (activeBroker !== null && !reload) {
      return;
    }
    if (activeBroker !== null) {
      destroyActiveBrokerForLifecycle();
    }
    mount(options, false);
  });
  engine.addEventListener("__RemoveComponents", destroyActiveBrokerForLifecycle);
  engine.addEventListener("__SSRHydrate", () => {
    throw new Error("LynxElementBridge does not support SSR hydration");
  });
  lynxApi.getNative().addEventListener("__DestroyLifetime", destroyActiveBrokerForLifecycle);
}
