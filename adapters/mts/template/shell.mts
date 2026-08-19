import { createLynxFiberHost } from "../src/lynx-fiber-host.mts";
import { installYewLynxShell } from "../src/shell-core.js";

installYewLynxShell({
  lynxApi: lynx,
  createPage: __CreatePage,
  getElementUniqueId: __GetElementUniqueID,
  createHost: createLynxFiberHost,
});
