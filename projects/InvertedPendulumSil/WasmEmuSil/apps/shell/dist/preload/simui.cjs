"use strict";

// src/preload/simui.ts
var import_electron = require("electron");
import_electron.contextBridge.exposeInMainWorld("ipemuEngine", {
  endpoint: () => import_electron.ipcRenderer.invoke("simui:endpoint"),
  restartEngine: () => import_electron.ipcRenderer.invoke("simui:restart-engine"),
  onEndpointChanged: (cb) => {
    const h = (_e, ep) => cb(ep);
    import_electron.ipcRenderer.on("simui:endpoint-changed", h);
    return () => import_electron.ipcRenderer.removeListener("simui:endpoint-changed", h);
  }
});
//# sourceMappingURL=simui.cjs.map
