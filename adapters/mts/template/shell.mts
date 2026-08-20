import { createLynxFiberHost } from "../src/lynx-fiber-host.mts";
import { installLynxElementBridgeShell } from "../src/shell-core.js";

installLynxElementBridgeShell({
  lynxApi: lynx,
  createPage: __CreatePage,
  getElementUniqueId: __GetElementUniqueID,
  createHost: createLynxFiberHost,
});
