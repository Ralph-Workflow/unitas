import { spawn } from "node:child_process";
import { existsSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

const DEFAULT_MIN_FRAMES = 60;
const DEFAULT_TIMEOUT_SECONDS = 30;
const POLL_INTERVAL_MS = 25;
const CHROME_START_TIMEOUT_MS = 20000;
const CHROME_STOP_TIMEOUT_MS = 2000;
const CHROME_CANDIDATES = ["/Applications/Google Chrome.app/Contents/MacOS/Google Chrome"];
const STATE_EXPRESSION = [
  "(() => {",
  "  const state = window.__unitas;",
  "  const now = typeof performance === 'undefined' ? 0 : performance.now();",
  "  if (!state) {",
  "    return JSON.stringify({ present: false, booted: false, frameCount: 0, error: null, now });",
  "  }",
  "  return JSON.stringify({",
  "    present: true,",
  "    booted: state.booted === true,",
  "    frameCount: Number(state.frameCount) || 0,",
  "    error: typeof state.error === 'string' ? state.error : null,",
  "    now,",
  "  });",
  "})()",
].join("\n");

function sleep(milliseconds) {
  return new Promise((resolve) => {
    setTimeout(resolve, milliseconds);
  });
}

function parseArgs(argv) {
  const options = {
    url: "",
    screenshot: "",
    consoleLog: "",
    chrome: "",
    minFrames: DEFAULT_MIN_FRAMES,
    timeoutSeconds: DEFAULT_TIMEOUT_SECONDS,
  };
  for (let index = 0; index < argv.length; index += 1) {
    const argument = argv[index];
    const value = () => {
      index += 1;
      const taken = argv[index];
      if (taken === undefined) {
        throw new Error(`missing value for ${argument}`);
      }
      return taken;
    };
    if (argument === "--url") {
      options.url = value();
    } else if (argument === "--screenshot") {
      options.screenshot = value();
    } else if (argument === "--console-log") {
      options.consoleLog = value();
    } else if (argument === "--chrome") {
      options.chrome = value();
    } else if (argument === "--min-frames") {
      options.minFrames = Number.parseInt(value(), 10);
    } else if (argument === "--timeout-seconds") {
      options.timeoutSeconds = Number.parseInt(value(), 10);
    } else {
      throw new Error(`unknown argument ${argument}`);
    }
  }
  if (!options.url) {
    throw new Error("--url is required");
  }
  if (!Number.isFinite(options.minFrames) || options.minFrames < 1) {
    throw new Error("--min-frames must be a positive number");
  }
  if (!Number.isFinite(options.timeoutSeconds) || options.timeoutSeconds < 1) {
    throw new Error("--timeout-seconds must be a positive number");
  }
  return options;
}

function resolveChrome(explicit) {
  if (explicit) {
    if (existsSync(explicit)) {
      return { path: explicit, reason: "" };
    }
    return { path: "", reason: `the Chrome binary ${explicit} does not exist` };
  }
  const fromEnvironment = process.env.CHROME_PATH ?? "";
  if (fromEnvironment) {
    if (existsSync(fromEnvironment)) {
      return { path: fromEnvironment, reason: "" };
    }
    return {
      path: "",
      reason: `CHROME_PATH points at ${fromEnvironment}, which does not exist`,
    };
  }
  for (const candidate of CHROME_CANDIDATES) {
    if (existsSync(candidate)) {
      return { path: candidate, reason: "" };
    }
  }
  return {
    path: "",
    reason:
      "no Chrome binary was found: set CHROME_PATH to a Chrome executable, " +
      `or install ${CHROME_CANDIDATES[0]}`,
  };
}

function chromeArguments(userDataDir) {
  return [
    "--headless=new",
    "--remote-debugging-port=0",
    `--user-data-dir=${userDataDir}`,
    "--no-first-run",
    "--no-default-browser-check",
    "--disable-background-networking",
    "--disable-component-update",
    "--disable-default-apps",
    "--disable-extensions",
    "--disable-sync",
    "--mute-audio",
    "--window-size=1280,720",
    "about:blank",
  ];
}

class DevtoolsClient {
  constructor(endpoint) {
    this.endpoint = endpoint;
    this.nextId = 1;
    this.pending = new Map();
    this.listeners = [];
    this.socket = null;
  }

  async connect() {
    const socket = new WebSocket(this.endpoint);
    this.socket = socket;
    await new Promise((resolve, reject) => {
      socket.addEventListener("open", () => resolve(), { once: true });
      socket.addEventListener(
        "error",
        () => reject(new Error("the DevTools websocket failed to open")),
        { once: true },
      );
    });
    socket.addEventListener("message", (event) => {
      this.handleMessage(event.data);
    });
  }

  handleMessage(data) {
    let message;
    try {
      message = JSON.parse(data);
    } catch {
      return;
    }
    if (typeof message.id === "number") {
      const entry = this.pending.get(message.id);
      if (entry === undefined) {
        return;
      }
      this.pending.delete(message.id);
      if (message.error) {
        entry.reject(new Error(String(message.error.message ?? "unknown DevTools error")));
      } else {
        entry.resolve(message.result ?? {});
      }
      return;
    }
    for (const listener of this.listeners) {
      listener(message);
    }
  }

  on(listener) {
    this.listeners.push(listener);
  }

  send(method, params = {}, sessionId = "") {
    const id = this.nextId;
    this.nextId += 1;
    const payload = { id, method, params };
    if (sessionId) {
      payload.sessionId = sessionId;
    }
    return new Promise((resolve, reject) => {
      this.pending.set(id, { resolve, reject });
      this.socket.send(JSON.stringify(payload));
    });
  }

  close() {
    if (this.socket === null) {
      return;
    }
    try {
      this.socket.close();
    } catch {
      return;
    }
  }
}

async function readEndpoint(chromeProcess, userDataDir) {
  const portFile = join(userDataDir, "DevToolsActivePort");
  const deadline = Date.now() + CHROME_START_TIMEOUT_MS;
  while (Date.now() < deadline) {
    if (chromeProcess.exitCode !== null) {
      throw new Error(
        `Chrome exited with code ${chromeProcess.exitCode} before it published a DevTools endpoint`,
      );
    }
    if (existsSync(portFile)) {
      const lines = readFileSync(portFile, "utf8").split("\n");
      const port = Number.parseInt(lines[0] ?? "", 10);
      const path = (lines[1] ?? "").trim();
      if (Number.isFinite(port) && port > 0 && path) {
        return `ws://127.0.0.1:${port}${path}`;
      }
    }
    await sleep(POLL_INTERVAL_MS);
  }
  throw new Error(
    `Chrome published no DevTools endpoint within ${CHROME_START_TIMEOUT_MS} ms`,
  );
}

function argumentText(argument) {
  if (argument === null || argument === undefined) {
    return "";
  }
  if (argument.type === "string") {
    return argument.value;
  }
  if ("value" in argument) {
    return JSON.stringify(argument.value);
  }
  return argument.description ?? argument.className ?? argument.type;
}

function collectPageEvents(message, sessionId, consoleMessages, consoleErrors, exceptions) {
  if (message.sessionId !== sessionId) {
    return;
  }
  if (message.method === "Runtime.consoleAPICalled") {
    const type = String(message.params?.type ?? "log");
    const text = (message.params?.args ?? []).map(argumentText).join(" ");
    consoleMessages.push({ type, text });
    if (type === "error" || type === "assert") {
      consoleErrors.push({ type, text });
    }
    return;
  }
  if (message.method === "Runtime.exceptionThrown") {
    const details = message.params?.exceptionDetails ?? {};
    const described = details.exception?.description ?? details.text ?? "unknown exception";
    exceptions.push(String(described));
  }
}

async function readPageState(client, sessionId) {
  const evaluation = await client.send(
    "Runtime.evaluate",
    { expression: STATE_EXPRESSION, returnByValue: true },
    sessionId,
  );
  const raw = evaluation?.result?.value;
  if (typeof raw !== "string") {
    return { present: false, booted: false, frameCount: 0, error: null, now: 0 };
  }
  try {
    const parsed = JSON.parse(raw);
    return {
      present: parsed.present === true,
      booted: parsed.booted === true,
      frameCount: Number(parsed.frameCount) || 0,
      error: typeof parsed.error === "string" ? parsed.error : null,
      now: Number(parsed.now) || 0,
    };
  } catch {
    return { present: false, booted: false, frameCount: 0, error: null, now: 0 };
  }
}

function killChrome(chromeProcess) {
  if (chromeProcess.exitCode !== null || chromeProcess.signalCode !== null) {
    return;
  }
  try {
    process.kill(-chromeProcess.pid, "SIGKILL");
  } catch {
    chromeProcess.kill("SIGKILL");
    return;
  }
  try {
    chromeProcess.kill("SIGKILL");
  } catch {
    return;
  }
}

async function stopChrome(chromeProcess) {
  if (chromeProcess.exitCode !== null) {
    return;
  }
  const exited = new Promise((resolve) => {
    chromeProcess.once("exit", resolve);
  });
  killChrome(chromeProcess);
  await Promise.race([exited, sleep(CHROME_STOP_TIMEOUT_MS)]);
}

function writeConsoleLog(path, consoleMessages, exceptions, chromeOutput) {
  if (!path) {
    return;
  }
  const lines = [];
  for (const message of consoleMessages) {
    lines.push(`[${message.type}] ${message.text}`);
  }
  for (const exception of exceptions) {
    lines.push(`[exception] ${exception}`);
  }
  for (const chunk of chromeOutput) {
    lines.push(`[chrome] ${chunk.trimEnd()}`);
  }
  writeFileSync(path, `${lines.join("\n")}\n`);
}

async function drivePage(options, client, sessionId, observations) {
  const deadline = Date.now() + options.timeoutSeconds * 1000;
  let state = { present: false, booted: false, frameCount: 0, error: null, now: 0 };
  while (Date.now() < deadline) {
    state = await readPageState(client, sessionId);
    if (state.frameCount > 0 && observations.firstFrameCount === null) {
      observations.firstFrameCount = state.frameCount;
      observations.firstFrameNow = state.now;
    }
    if (state.error !== null) {
      observations.reason = `the web player reported an error: ${state.error}`;
      break;
    }
    if (state.booted && state.frameCount >= options.minFrames) {
      break;
    }
    await sleep(POLL_INTERVAL_MS);
  }
  observations.frameCount = state.frameCount;
  observations.booted = state.booted;
  observations.error = state.error;
  if (observations.reason === "" && observations.firstFrameCount === null) {
    observations.reason =
      `the page exposed no advancing frame counter within ${options.timeoutSeconds}s`;
    return;
  }
  if (observations.reason !== "") {
    return;
  }
  if (!state.booted) {
    observations.reason = `the web player never reported a boot within ${options.timeoutSeconds}s`;
    return;
  }
  if (state.frameCount < options.minFrames) {
    observations.reason =
      `the web player advanced ${state.frameCount} frames; ` +
      `at least ${options.minFrames} are required`;
    return;
  }
  const finalNow = state.now > 0 ? state.now : Date.now();
  const startedNow = observations.firstFrameNow > 0 ? observations.firstFrameNow : finalNow;
  observations.elapsedSeconds = Math.max((finalNow - startedNow) / 1000, 0);
  const advanced = state.frameCount - observations.firstFrameCount;
  observations.framesPerSecond =
    observations.elapsedSeconds > 0 ? advanced / observations.elapsedSeconds : 0;
}

async function captureScreenshot(client, sessionId, path, observations) {
  try {
    const capture = await client.send("Page.captureScreenshot", { format: "png" }, sessionId);
    const data = capture?.data ?? "";
    if (!data) {
      observations.screenshotError = "the browser captured no screenshot data";
      return;
    }
    writeFileSync(path, Buffer.from(data, "base64"));
    observations.screenshot = path;
  } catch (error) {
    observations.screenshotError = `the browser refused the screenshot: ${error.message}`;
  }
}

async function runDriver(options) {
  const started = Date.now();
  const chrome = resolveChrome(options.chrome);
  const observations = {
    reason: "",
    frameCount: 0,
    booted: false,
    error: null,
    firstFrameCount: null,
    firstFrameNow: 0,
    elapsedSeconds: 0,
    framesPerSecond: 0,
    screenshot: "",
    screenshotError: "",
  };
  const consoleMessages = [];
  const consoleErrors = [];
  const exceptions = [];
  const chromeOutput = [];
  if (chrome.path === "") {
    observations.reason = chrome.reason;
    return { observations, consoleMessages, consoleErrors, exceptions, chromeOutput, chrome: chrome.path };
  }
  const userDataDir = mkdtempSync(join(tmpdir(), "unitas-web-lane-"));
  let chromeProcess = null;
  let client = null;
  try {
    chromeProcess = spawn(chrome.path, chromeArguments(userDataDir), {
      detached: true,
      stdio: ["ignore", "pipe", "pipe"],
    });
    chromeProcess.stdout.on("data", (chunk) => {
      chromeOutput.push(String(chunk));
    });
    chromeProcess.stderr.on("data", (chunk) => {
      chromeOutput.push(String(chunk));
    });
    const endpoint = await readEndpoint(chromeProcess, userDataDir);
    client = new DevtoolsClient(endpoint);
    await client.connect();
    const target = await client.send("Target.createTarget", { url: "about:blank" });
    const attached = await client.send("Target.attachToTarget", {
      targetId: target.targetId,
      flatten: true,
    });
    const sessionId = String(attached.sessionId);
    client.on((message) => {
      collectPageEvents(message, sessionId, consoleMessages, consoleErrors, exceptions);
    });
    await client.send("Runtime.enable", {}, sessionId);
    await client.send("Page.enable", {}, sessionId);
    await client.send("Page.navigate", { url: options.url }, sessionId);
    await drivePage(options, client, sessionId, observations);
    await captureScreenshot(client, sessionId, options.screenshot, observations);
    if (observations.reason === "" && observations.screenshotError !== "") {
      observations.reason = observations.screenshotError;
    }
    if (observations.reason === "" && consoleErrors.length > 0) {
      observations.reason =
        `the browser console reported ${consoleErrors.length} error(s): ` +
        consoleErrors.map((entry) => entry.text).join(" | ");
    }
    if (observations.reason === "" && exceptions.length > 0) {
      observations.reason =
        `the page raised ${exceptions.length} uncaught exception(s): ` +
        exceptions.join(" | ");
    }
  } catch (error) {
    observations.reason = `the browser driver failed: ${error.message}`;
  } finally {
    if (client !== null) {
      client.close();
    }
    if (chromeProcess !== null) {
      await stopChrome(chromeProcess);
    }
    rmSync(userDataDir, { recursive: true, force: true });
  }
  return {
    observations,
    consoleMessages,
    consoleErrors,
    exceptions,
    chromeOutput,
    chrome: chrome.path,
    durationSeconds: (Date.now() - started) / 1000,
  };
}

function verdictOf(result, options) {
  const observations = result.observations;
  return {
    ok: observations.reason === "",
    reason: observations.reason,
    url: options.url,
    chrome: result.chrome,
    frame_count: observations.frameCount,
    min_frames: options.minFrames,
    first_frame_count: observations.firstFrameCount,
    elapsed_seconds: observations.elapsedSeconds,
    frames_per_second: observations.framesPerSecond,
    booted: observations.booted,
    error: observations.error,
    console_error_count: result.consoleErrors.length,
    console_errors: result.consoleErrors,
    exception_count: result.exceptions.length,
    exceptions: result.exceptions,
    console: result.consoleMessages,
    screenshot: observations.screenshot,
    screenshot_error: observations.screenshotError,
    console_log: options.consoleLog,
    duration_seconds: result.durationSeconds ?? 0,
  };
}

async function main() {
  let options;
  try {
    options = parseArgs(process.argv.slice(2));
  } catch (error) {
    process.stdout.write(
      `${JSON.stringify({ ok: false, reason: error.message, frame_count: 0, console_error_count: 0 })}\n`,
    );
    process.exitCode = 2;
    return;
  }
  const result = await runDriver(options);
  writeConsoleLog(
    options.consoleLog,
    result.consoleMessages,
    result.exceptions,
    result.chromeOutput,
  );
  const verdict = verdictOf(result, options);
  process.stdout.write(`${JSON.stringify(verdict)}\n`);
  process.exitCode = verdict.ok ? 0 : 1;
}

await main();
