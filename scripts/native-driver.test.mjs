import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { test } from "node:test";

const shell = await readFile(new URL("./app-drive.sh", import.meta.url), "utf8");
const jxa = await readFile(new URL("./ax-drive.js", import.meta.url), "utf8");
const app = await readFile(new URL("../apps/web/src/shell/App.tsx", import.meta.url), "utf8");

test("fill supports WebKit text inputs and verifies the text landed", () => {
  assert.match(shell, /ax focus \"\$field\"/);
  assert.doesNotMatch(shell, /OPT_ROLE=\"\$\{OPT_ROLE:-AXTextField\}\" ax click/);
  // The typed text must be confirmed, not assumed: a stolen global keystroke is
  // invisible otherwise.
  assert.match(shell, /did not stick/);
  assert.match(jxa, /AXTextArea/);
  assert.match(jxa, /commandFocus/);
  assert.match(jxa, /AXFocused/);
});

test("an AXTextArea is focused with a real mouse click, not AXPress", () => {
  // WebKit ignores AXPress for textarea focus, so the driver clicks the element's
  // on-screen center through the same event path a user does. A native date
  // input (AXDateTimeArea) behaves the same way and shares the click path.
  assert.match(jxa, /function coordinateClick/);
  assert.match(jxa, /CGEventCreateMouseEvent/);
  assert.match(jxa, /kCGHIDEventTap/);
  assert.match(jxa, /AXTextArea\"[\s\S]*coordinateClick/);
  assert.match(jxa, /AXDateTimeArea\"[\s\S]*coordinateClick/);
});

test("a date segment is set with AXIncrement/AXDecrement, not by writing AXValue", () => {
  // A controlled <input type="date"> cannot be set by writing AXValue (WebKit
  // accepts the write and drops it) and synthetic digits land unreliably, so the
  // driver moves the segment incrementor, which fires the real DOM change event.
  assert.match(jxa, /function commandStep/);
  assert.match(jxa, /AXIncrement/);
  assert.match(jxa, /AXDecrement/);
  assert.match(shell, /step\s+\[-s ID\] --steps N <name>/);
  assert.match(shell, /--steps/);
});

test("native popup selection clicks the exposed menu item and verifies the value", () => {
  assert.match(shell, /choose \[-s ID\] <name> <opt>/);
  // The option is chosen by clicking its AXMenuItem; typing the label is only a
  // fallback for a popup that exposes no items.
  assert.match(shell, /AXMenuItem\" ax click \"\$option\"/);
  assert.match(shell, /key code 36/);
  assert.match(shell, /ax value \"\$control\"/);
  assert.match(jxa, /commandValue/);
});

test("driver reports unsupported text and popup controls clearly", () => {
  assert.match(jxa, /not a supported text input/);
  assert.match(shell, /unsupported popup control/);
});

test("footer version comes from Tauri package metadata", () => {
  assert.match(app, /import \{ getVersion \} from "@tauri-apps\/api\/app"/);
  assert.match(app, /getVersion\(\)/);
  assert.match(app, /version=\{appVersion\}/);
  assert.doesNotMatch(app, /version=\"0\.9\.22\"/);
});
