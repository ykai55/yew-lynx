import { createReadStream } from "node:fs";
import { stat } from "node:fs/promises";
import http from "node:http";
import { createRequire } from "node:module";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";

const require = createRequire(import.meta.url);
const qrcode = require("qrcode-terminal");

const adapterDirectory = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const defaultBundlePath = path.join(adapterDirectory, "dist", "yew-lynx-counter.lynx.bundle");

const options = {
  bundlePath: defaultBundlePath,
  host: process.env.HOST || "127.0.0.1",
  printQr: process.env.NO_QR !== "1",
  port: Number.parseInt(process.env.PORT || "4173", 10),
};

const arguments_ = process.argv.slice(2);
for (let index = 0; index < arguments_.length; index += 1) {
  const argument = arguments_[index];
  const value = arguments_[index + 1];
  if (argument === "--host" && value) {
    options.host = value;
    index += 1;
  } else if (argument === "--port" && value) {
    options.port = Number.parseInt(value, 10);
    index += 1;
  } else if (argument === "--bundle" && value) {
    options.bundlePath = path.resolve(adapterDirectory, value);
    index += 1;
  } else if (argument === "--no-qr") {
    options.printQr = false;
  } else {
    throw new Error(`unsupported argument: ${argument}`);
  }
}

if (!Number.isInteger(options.port) || options.port < 1 || options.port > 65535) {
  throw new Error(`invalid port: ${options.port}`);
}

const bundleStats = await stat(options.bundlePath).catch((error) => {
  if (error && error.code === "ENOENT") {
    throw new Error(
      `missing bundle: ${path.relative(adapterDirectory, options.bundlePath)}. Run npm run build first.`,
    );
  }
  throw error;
});
if (!bundleStats.isFile() || bundleStats.size === 0) {
  throw new Error(`bundle is not a non-empty file: ${options.bundlePath}`);
}

const bundleName = path.basename(options.bundlePath);
const bundleRoute = `/${bundleName}`;
const indexRoute = "/";

function localNetworkAddresses() {
  const addresses = [];
  for (const networkInterface of Object.values(os.networkInterfaces())) {
    for (const address of networkInterface || []) {
      if (address.family === "IPv4" && !address.internal) {
        addresses.push(address.address);
      }
    }
  }
  return addresses;
}

function sendJson(request, response, statusCode, payload) {
  const body = `${JSON.stringify(payload, null, 2)}\n`;
  response.writeHead(statusCode, {
    "Access-Control-Allow-Origin": "*",
    "Cache-Control": "no-store",
    "Content-Length": Buffer.byteLength(body),
    "Content-Type": "application/json; charset=utf-8",
  });
  response.end(request.method === "HEAD" ? undefined : body);
}

function handles(method) {
  return method === "GET" || method === "HEAD";
}

function printQrCode(label, url) {
  if (!options.printQr) {
    return;
  }
  console.log(`${label} QR:`);
  qrcode.generate(url, { small: true }, (qrCode) => {
    console.log(qrCode);
  });
}

const server = http.createServer((request, response) => {
  const requestUrl = new URL(request.url || "/", "http://localhost");
  if (handles(request.method) && requestUrl.pathname === indexRoute) {
    sendJson(request, response, 200, {
      bundle: bundleRoute,
      bytes: bundleStats.size,
    });
    return;
  }
  if (handles(request.method) && requestUrl.pathname === bundleRoute) {
    response.writeHead(200, {
      "Access-Control-Allow-Origin": "*",
      "Cache-Control": "no-store",
      "Content-Length": bundleStats.size,
      "Content-Type": "application/octet-stream",
    });
    if (request.method === "HEAD") {
      response.end();
    } else {
      createReadStream(options.bundlePath).pipe(response);
    }
    return;
  }
  response.writeHead(404, {
    "Access-Control-Allow-Origin": "*",
    "Cache-Control": "no-store",
    "Content-Type": "text/plain; charset=utf-8",
  });
  response.end("Not Found\n");
});

server.listen(options.port, options.host, () => {
  const printableHost = options.host === "0.0.0.0" ? "127.0.0.1" : options.host;
  const bundleUrl = `http://${printableHost}:${options.port}${bundleRoute}`;
  console.log(`Serving ${path.relative(adapterDirectory, options.bundlePath)}`);
  console.log(`Bundle URL: ${bundleUrl}`);
  printQrCode("Bundle URL", bundleUrl);
  if (options.host === "0.0.0.0") {
    for (const address of localNetworkAddresses()) {
      const lanUrl = `http://${address}:${options.port}${bundleRoute}`;
      console.log(`LAN URL: ${lanUrl}`);
      printQrCode("LAN URL", lanUrl);
    }
  }
});

process.on("SIGINT", () => {
  server.close(() => process.exit(0));
});
