// JXA accessibility driver for a running Aevio window. Invoked only by
// scripts/app-drive.sh. See that script and .agents/skills/drive-app/SKILL.md.
//
// Why this exists: Aevio renders through WKWebView, and System Events'
// `entire contents` stops at the web view boundary (it reports a single empty
// AXGroup). WebKit *does* publish the full web accessibility tree, it is just
// only reachable by walking AXUIElement references directly, which is what
// this file does. That gives coordinate-free, name-based driving of the real
// OS webview: `AXPress` on a web AXButton dispatches a genuine DOM click.
//
// All input arrives via environment variables so arbitrary label text never has
// to survive osascript argument quoting:
//   AX_PID     target process id (required)
//   AX_CMD     tree | find | click | windowid (required)
//   AX_TARGET  accessible name to match (find/click/focus)
//   AX_ROLE    optional AX role filter, e.g. AXRadioButton
//   AX_INDEX   0-based pick when a match is ambiguous
//   AX_WINDOW_TITLE optional exact window title (default: prefer Aevio main)
//   AX_FULL    "1" to print every node in `tree`, not just named/actionable ones

ObjC.import("ApplicationServices");
ObjC.import("CoreGraphics");
ObjC.import("Foundation");

// JXA's console.log writes to stderr and there is no built-in $.exit, so both
// are wired up by hand: results must reach stdout for the shell to grep, and
// the exit status is how app-drive.sh knows whether a click landed.
ObjC.bindFunction("exit", ["void", ["int"]]);

function write(handle, text) {
  handle.writeData(
    $.NSString.alloc.initWithUTF8String(text + "\n").dataUsingEncoding($.NSUTF8StringEncoding),
  );
}
function emit(text) {
  write($.NSFileHandle.fileHandleWithStandardOutput, text);
}
function warn(text) {
  write($.NSFileHandle.fileHandleWithStandardError, text);
}

var MAX_DEPTH = 24;

// Roles that represent something a user can act on. Used both to decide what
// `tree` is worth printing and what `click`/`focus` are allowed to match.
var ACTIONABLE = {
  AXButton: 1,
  AXCheckBox: 1,
  AXComboBox: 1,
  AXDateTimeArea: 1,
  AXDisclosureTriangle: 1,
  AXIncrementor: 1,
  AXLink: 1,
  AXMenuButton: 1,
  AXMenuItem: 1,
  AXPopUpButton: 1,
  AXRadioButton: 1,
  AXSearchField: 1,
  AXSlider: 1,
  AXTabButton: 1,
  AXTextArea: 1,
  AXTextField: 1,
};

function env(name) {
  var value = $.NSProcessInfo.processInfo.environment.objectForKey(name);
  return value ? value.js : "";
}

function rawAttr(element, name) {
  var out = Ref();
  if ($.AXUIElementCopyAttributeValue(element, $(name), out) !== 0) return null;
  return out[0];
}

function strAttr(element, name) {
  var value = rawAttr(element, name);
  if (!value) return "";
  try {
    var unwrapped = ObjC.castRefToObject(value).js;
    return typeof unwrapped === "string" ? unwrapped : String(unwrapped);
  } catch (e) {
    return "";
  }
}

function boolAttr(element, name) {
  var value = rawAttr(element, name);
  if (!value) return false;
  try { return ObjC.castRefToObject(value).boolValue; } catch (e) { return false; }
}

function children(element) {
  var kids = rawAttr(element, "AXChildren");
  return kids ? ObjC.castRefToObject(kids) : null;
}

// The label a human would use for this node: buttons carry AXTitle, links and
// aria-labelled controls carry AXDescription.
function labelOf(element) {
  return strAttr(element, "AXTitle") || strAttr(element, "AXDescription");
}

// What `click`/`find` match on. Plain text nodes have no label, so their content
// stands in for one.
function nameOf(element) {
  return labelOf(element) || strAttr(element, "AXValue");
}

function walk(element, depth, visit) {
  if (depth > MAX_DEPTH) return;
  var printed = visit(element, depth) ? 1 : 0;
  var kids = children(element);
  if (!kids) return;
  for (var i = 0; i < kids.count; i++) walk(kids.objectAtIndex(i), depth + printed, visit);
}

function rootWindow() {
  var pid = parseInt(env("AX_PID"), 10);
  if (!pid) {
    warn("ax-drive: AX_PID is required");
    return null;
  }
  var app = $.AXUIElementCreateApplication(pid);
  var windows = rawAttr(app, "AXWindows");
  if (!windows) {
    warn("ax-drive: no accessible windows for pid " + pid);
    return null;
  }
  var list = ObjC.castRefToObject(windows);
  if (list.count === 0) {
    warn("ax-drive: process " + pid + " has no window yet");
    return null;
  }
  var wanted = env("AX_WINDOW_TITLE") || "Aevio";
  for (var i = 0; i < list.count; i++) {
    var candidate = list.objectAtIndex(i);
    if (strAttr(candidate, "AXTitle") === wanted) return candidate;
  }
  if (env("AX_WINDOW_TITLE")) {
    warn("ax-drive: no accessible window named " + wanted);
    return null;
  }
  return list.objectAtIndex(0);
}

function clamp(text) {
  var out = text.replace(/\s+/g, " ").trim();
  return out.length > 90 ? out.slice(0, 90) + "…" : out;
}

function describe(element, depth) {
  var role = strAttr(element, "AXRole").replace(/^AX/, "");
  var label = clamp(labelOf(element));
  var value = clamp(strAttr(element, "AXValue"));
  var indent = new Array(depth + 1).join("  ");
  // A labelled control shows both, so the current contents of a text field are
  // visible without a screenshot; an unlabelled text node shows its value alone.
  var text = label || value;
  if (label && value && value !== label) text = label + ' = "' + value + '"';
  return indent + "[" + role + "] " + text;
}

function commandTree(window) {
  var full = env("AX_FULL") === "1";
  var lines = [];
  walk(window, 0, function (element, depth) {
    var role = strAttr(element, "AXRole");
    if (!full && !ACTIONABLE[role] && !nameOf(element)) return false;
    lines.push(describe(element, depth));
    return true;
  });
  emit(lines.join("\n"));
  return 0;
}

// Match in tiers so an exact label always beats an incidental substring hit:
// exact, then case-insensitive exact, then case-insensitive substring.
function matches(window, role, target) {
  var tiers = [[], [], []];
  var wanted = target.toLowerCase();
  walk(window, 0, function (element) {
    var elementRole = strAttr(element, "AXRole");
    if (role) {
      if (elementRole !== role) return false;
    } else if (!ACTIONABLE[elementRole]) {
      return false;
    }
    var name = nameOf(element).replace(/\s+/g, " ").trim();
    if (!name) return false;
    var lower = name.toLowerCase();
    if (name === target) tiers[0].push([element, elementRole, name]);
    else if (lower === wanted) tiers[1].push([element, elementRole, name]);
    else if (lower.indexOf(wanted) !== -1) tiers[2].push([element, elementRole, name]);
    return false;
  });
  for (var i = 0; i < tiers.length; i++) if (tiers[i].length) return tiers[i];
  return [];
}

function resolve(window, role, target) {
  var found = matches(window, role, target);
  if (found.length === 0) {
    warn('ax-drive: no actionable element named "' + target + '"');
    warn("run `app-drive.sh tree` to see what is on screen");
    return null;
  }
  if (found.length === 1) return found[0];

  var index = env("AX_INDEX");
  if (index !== "") {
    var picked = found[parseInt(index, 10)];
    if (!picked) {
      warn("ax-drive: AX_INDEX " + index + " is out of range (" + found.length + " matches)");
      return null;
    }
    return picked;
  }
  warn('ax-drive: "' + target + '" is ambiguous - ' + found.length + " matches:");
  for (var i = 0; i < found.length; i++) {
    warn("  [" + i + "] " + found[i][1].replace(/^AX/, "") + " " + found[i][2]);
  }
  warn("re-run with --index N to pick one");
  return null;
}

function commandFind(window) {
  var hit = resolve(window, env("AX_ROLE"), env("AX_TARGET"));
  if (!hit) return 1;
  emit(hit[1].replace(/^AX/, "") + " " + hit[2]);
  return 0;
}

function commandClick(window) {
  var hit = resolve(window, env("AX_ROLE"), env("AX_TARGET"));
  if (!hit) return 1;
  // A WebKit AXTextArea ignores AXPress for focus purposes (see commandFocus),
  // and an AXDateTimeArea (a native <input type="date">) likewise does not
  // respond to AXPress: both need a real mouse click at their center, exactly as
  // a user would. Every other control keeps using AXPress, which dispatches a
  // genuine DOM click through the webview.
  if (hit[1] === "AXTextArea" || hit[1] === "AXDateTimeArea") {
    if (!coordinateClick(hit[0])) return 1;
    emit("clicked " + hit[1].replace(/^AX/, "") + " " + hit[2]);
    return 0;
  }
  var code = $.AXUIElementPerformAction(hit[0], $("AXPress"));
  if (code !== 0) {
    warn('ax-drive: AXPress on "' + hit[2] + '" failed (AXError ' + code + ")");
    return 1;
  }
  // A press on a text input already moves focus (and on an AXTextField flips
  // AXFocused true). Writing AXFocused afterwards would clobber that focus, so
  // only nudge it when the press demonstrably left it unclaimed.
  if (hit[1] === "AXTextField" && !boolAttr(hit[0], "AXFocused")) {
    $.AXUIElementSetAttributeValue(hit[0], $("AXFocused"), $.kCFBooleanTrue);
  }
  emit("clicked " + hit[1].replace(/^AX/, "") + " " + hit[2]);
  return 0;
}

// What the OS actually does, verified against a live WKWebView:
//   - AXPress is the reliable focus mechanism for AXTextField and AXSearchField.
//     On an AXTextField it immediately flips AXFocused to true.
//   - AXPress does NOT focus a WebKit AXTextArea: AXPress returns AXError 0 and
//     flips nothing, so a following keystroke lands nowhere. A genuine mouse
//     click at the element's center DOES focus it, so an AXTextArea is focused
//     with coordinateClick, not AXPress.
//   - An AXTextArea reports AXFocused=false even once its DOM focus is real, so
//     its read-back must not be trusted as a negative; success is proven by the
//     press/click landing rather than by the attribute.
//   - An AXDateTimeArea (native <input type="date">) behaves like an AXTextArea:
//     AXPress does nothing, a real coordinate click focuses it.
//   - Writing AXFocused=true is accepted (AXError 0) but WebKit clobbers the
//     focus the click had already established, so it is never written for a
//     text input; it is only a last resort for the rare control AXPress leaves
//     unclaimed.
function commandFocus(window) {
  var hit = resolve(window, "", env("AX_TARGET"));
  if (!hit || (hit[1] !== "AXTextField" && hit[1] !== "AXTextArea" && hit[1] !== "AXSearchField" && hit[1] !== "AXDateTimeArea")) {
    warn('ax-drive: "' + env("AX_TARGET") + '" is not a supported text input');
    return 1;
  }
  if (hit[1] === "AXTextArea" || hit[1] === "AXDateTimeArea") {
    if (!coordinateClick(hit[0])) return 1;
    emit("focused " + hit[1].replace(/^AX/, "") + " " + hit[2] + " (click)");
    return 0;
  }
  var pressed = $.AXUIElementPerformAction(hit[0], $("AXPress"));
  if (pressed !== 0) {
    warn('ax-drive: AXPress on "' + hit[2] + '" failed (AXError ' + pressed + ")");
    return 1;
  }
  if (boolAttr(hit[0], "AXFocused")) {
    emit("focused " + hit[1].replace(/^AX/, "") + " " + hit[2]);
    return 0;
  }
  var written = $.AXUIElementSetAttributeValue(hit[0], $("AXFocused"), $.kCFBooleanTrue);
  if (written !== 0 || !boolAttr(hit[0], "AXFocused")) {
    warn('ax-drive: could not verify focus for "' + hit[2] + '" (AXError ' + written + ")");
    return 1;
  }
  emit("focused " + hit[1].replace(/^AX/, "") + " " + hit[2]);
  return 0;
}

function commandValue(window) {
  var hit = resolve(window, env("AX_ROLE"), env("AX_TARGET"));
  if (!hit) return 1;
  emit(strAttr(hit[0], "AXValue"));
  return 0;
}

// A stepper-like control (AXIncrementor, and the per-segment incrementors a
// native <input type="date"> exposes) accepts AXIncrement/AXDecrement. This is
// the only reliable way to move such a control: a date input is a controlled
// React field, its AXValue cannot be written (the write is accepted and
// silently dropped, exactly like a textarea), and synthetic digit keystrokes
// land inconsistently across the month/day/year segments. Driving the segment
// incrementor fires the real DOM input/change event React listens for.
// AX_STEPS is signed: positive increments, negative decrements.
function commandStep(window) {
  var hit = resolve(window, env("AX_ROLE"), env("AX_TARGET"));
  if (!hit) return 1;
  var steps = parseInt(env("AX_STEPS"), 10);
  if (!steps) {
    warn("ax-drive: AX_STEPS must be a non-zero integer");
    return 1;
  }
  var action = $(steps > 0 ? "AXIncrement" : "AXDecrement");
  var count = steps > 0 ? steps : -steps;
  for (var i = 0; i < count; i++) {
    var code = $.AXUIElementPerformAction(hit[0], action);
    if (code !== 0) {
      warn('ax-drive: ' + (steps > 0 ? "AXIncrement" : "AXDecrement") + ' on "' + hit[2] + '" failed (AXError ' + code + ")");
      return 1;
    }
  }
  emit((steps > 0 ? "incremented " : "decremented ") + hit[2] + " by " + count + " (now " + strAttr(hit[0], "AXValue") + ")");
  return 0;
}

// AXValue attributes (AXPosition, AXSize) do not project into JXA: the Ref
// comes back as an opaque object with no readable fields. Their ObjC
// `description` is stable and does carry the numbers, so parse that:
//   <AXValue 0x...> {value = x:2452.000000 y:765.000000 type = kAXValueCGPointType}
function parsePoint(description) {
  var match = /x:\s*(-?\d+(?:\.\d+)?)\s+y:\s*(-?\d+(?:\.\d+)?)/.exec(description || "");
  return match ? { x: parseFloat(match[1]), y: parseFloat(match[2]) } : null;
}
function parseSize(description) {
  var match = /w:\s*(-?\d+(?:\.\d+)?)\s+h:\s*(-?\d+(?:\.\d+)?)/.exec(description || "");
  return match ? { w: parseFloat(match[1]), h: parseFloat(match[2]) } : null;
}
function valueDescription(element, name) {
  var value = rawAttr(element, name);
  if (!value) return "";
  try { return ObjC.castRefToObject(value).description.js; } catch (e) { return ""; }
}
function centerOf(element) {
  var point = parsePoint(valueDescription(element, "AXPosition"));
  var size = parseSize(valueDescription(element, "AXSize"));
  if (!point || !size) return null;
  return { x: point.x + size.w / 2, y: point.y + size.h / 2 };
}

// A genuine mouse click at the element's on-screen center. AXPress on a WebKit
// textarea dispatches no usable DOM focus, but a real click does - which is the
// whole point: this reproduces exactly what a human does, through the same OS
// event path, instead of guessing at AX focus attributes the webview reports
// dishonestly. Coordinates are global (kCGHIDEventTap), so the app must already
// be frontmost and unoccluded at that point, which app-drive.sh guarantees.
function coordinateClick(element) {
  var center = centerOf(element);
  if (!center) {
    warn('ax-drive: could not read geometry for "' + env("AX_TARGET") + '"');
    return false;
  }
  var at = $.CGPointMake(center.x, center.y);
  function post(type) {
    var event = $.CGEventCreateMouseEvent(null, type, at, $.kCGMouseButtonLeft);
    $.CGEventPost($.kCGHIDEventTap, event);
  }
  post($.kCGEventMouseMoved);
  $.NSThread.sleepForTimeInterval(0.08);
  post($.kCGEventLeftMouseDown);
  $.NSThread.sleepForTimeInterval(0.06);
  post($.kCGEventLeftMouseUp);
  return true;
}

// The CGWindowID lets screencapture grab this exact window even when another
// agent's instance overlaps it, which capturing a screen rectangle cannot do.
function commandWindowId() {
  var pid = parseInt(env("AX_PID"), 10);
  var raw = $.CGWindowListCopyWindowInfo(
    $.kCGWindowListOptionOnScreenOnly | $.kCGWindowListExcludeDesktopElements,
    $.kCGNullWindowID,
  );
  var info = ObjC.deepUnwrap(ObjC.castRefToObject(raw));
  var wanted = env("AX_WINDOW_TITLE") || "Aevio";
  var fallback = null;
  for (var i = 0; i < info.length; i++) {
    // Layer 0 is a normal window; the tray icon and shadows live on other layers.
    if (info[i].kCGWindowOwnerPID === pid && info[i].kCGWindowLayer === 0) {
      if (info[i].kCGWindowName === wanted) { emit(String(info[i].kCGWindowNumber)); return 0; }
      if (fallback === null) fallback = info[i].kCGWindowNumber;
    }
  }
  if (fallback !== null && !env("AX_WINDOW_TITLE")) { emit(String(fallback)); return 0; }
  warn("ax-drive: no on-screen window for pid " + pid + " named " + wanted);
  return 1;
}

function main() {
  if (env("AX_CMD") === "windowid") return commandWindowId();
  var window = rootWindow();
  if (!window) return 1;
  var command = env("AX_CMD");
  if (command === "tree") return commandTree(window);
  if (command === "find") return commandFind(window);
  if (command === "click") return commandClick(window);
  if (command === "focus") return commandFocus(window);
  if (command === "value") return commandValue(window);
  if (command === "step") return commandStep(window);
  warn("ax-drive: unknown AX_CMD " + command);
  return 1;
}

$.exit(main());
