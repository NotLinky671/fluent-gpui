"""Drive the overlay probe with the real input queue and measure what happens.

The built-in tests draw frames and dispatch synthetic events; they cannot tell
whether a press on a panel really belongs to the panel, whether a tooltip shows
up on hover, whether a submenu waits for its delay, or whether a panel follows a
trigger that the wheel moved. This script does exactly those gestures against
`crates/fluent-gallery/examples/overlays_probe.rs` and reads the answer from
pixels and from the probe's own log.

Usage:

    python scripts/verify-overlays-windows.py [--exe target/debug/examples/overlays_probe.exe]

It exits non-zero if a check fails and writes `target/overlays-native.json`.
"""

from __future__ import annotations

import argparse
import ctypes
import ctypes.wintypes as wintypes
import json
import subprocess
import sys
import threading
import time
from pathlib import Path

from PIL import ImageGrab

user32 = ctypes.WinDLL("user32", use_last_error=True)

MOUSEEVENTF_LEFTDOWN = 0x0002
MOUSEEVENTF_LEFTUP = 0x0004
MOUSEEVENTF_WHEEL = 0x0800
KEYEVENTF_KEYUP = 0x0002
VK_ESCAPE = 0x1B
WHEEL_DELTA = 120


def enable_dpi_awareness() -> None:
    try:
        user32.SetProcessDpiAwarenessContext(ctypes.c_void_p(-4))
    except AttributeError:
        user32.SetProcessDPIAware()


def find_window(pid: int) -> int:
    """The probe's own window: the largest visible one that is not an IME helper."""
    found: list[tuple[int, int]] = []
    proc = ctypes.WINFUNCTYPE(wintypes.BOOL, wintypes.HWND, wintypes.LPARAM)

    def visit(hwnd, _):
        owner = wintypes.DWORD()
        user32.GetWindowThreadProcessId(hwnd, ctypes.byref(owner))
        if owner.value != pid or not user32.IsWindowVisible(hwnd):
            return True
        name = ctypes.create_unicode_buffer(64)
        user32.GetClassNameW(hwnd, name, 64)
        if name.value in ("IME", "MSCTFIME UI", "Default IME"):
            return True
        width, height = client_size(hwnd)
        if width >= 200 and height >= 200:
            found.append((width * height, hwnd))
        return True

    user32.EnumWindows(proc(visit), 0)
    if not found:
        raise RuntimeError(f"no visible window for pid {pid}")
    return max(found)[1]


def client_origin(hwnd: int) -> tuple[int, int]:
    """Where the client area starts on screen, in physical pixels."""
    rect = wintypes.RECT()
    user32.GetClientRect(hwnd, ctypes.byref(rect))
    origin = wintypes.POINT(0, 0)
    user32.ClientToScreen(hwnd, ctypes.byref(origin))
    return origin.x, origin.y


def client_size(hwnd: int) -> tuple[int, int]:
    rect = wintypes.RECT()
    user32.GetClientRect(hwnd, ctypes.byref(rect))
    return rect.right - rect.left, rect.bottom - rect.top


def scale_of(hwnd: int) -> float:
    try:
        return user32.GetDpiForWindow(hwnd) / 96.0
    except AttributeError:
        return 1.0


class Reader(threading.Thread):
    """Collect the probe's stdout without blocking the checks."""

    def __init__(self, stream):
        super().__init__(daemon=True)
        self.stream = stream
        self.lines: list[str] = []
        self.lock = threading.Lock()

    def run(self):
        for raw in self.stream:
            line = raw.strip()
            if line:
                with self.lock:
                    self.lines.append(line)

    def snapshot(self) -> list[str]:
        with self.lock:
            return list(self.lines)

    def wait_for(self, needle: str, timeout: float = 3.0) -> str | None:
        deadline = time.time() + timeout
        while time.time() < deadline:
            for line in self.snapshot():
                if needle in line:
                    return line
            time.sleep(0.03)
        return None

    def count(self, needle: str) -> int:
        return sum(1 for line in self.snapshot() if needle in line)


class Probe:
    def __init__(self, exe: Path, mode: str):
        self.mode = mode
        self.process = subprocess.Popen(
            [str(exe), mode],
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            text=True,
            encoding="utf-8",
            bufsize=1,
        )
        self.reader = Reader(self.process.stdout)
        self.reader.start()
        ready = self.reader.wait_for("window=", timeout=20)
        if ready is None:
            raise RuntimeError(f"the {mode} probe never reported its window")
        self.width, self.height = (float(v) for v in ready.split("=", 1)[1].split(","))
        deadline = time.time() + 20
        self.hwnd = 0
        while time.time() < deadline:
            try:
                self.hwnd = find_window(self.process.pid)
                break
            except RuntimeError:
                time.sleep(0.2)
        if not self.hwnd:
            raise RuntimeError(f"the {mode} probe never opened a window")
        self.scale = scale_of(self.hwnd)
        time.sleep(0.6)
        # The first press on a fresh window only activates it, never reaching the
        # widgets; give the probe that press on an empty spot up front.
        click(self.point((40, self.height - 40)))
        time.sleep(0.3)

    def anchors(self) -> dict[str, tuple[int, int]]:
        found: dict[str, tuple[int, int]] = {}
        deadline = time.time() + 5
        while time.time() < deadline:
            for line in self.reader.snapshot():
                if line.startswith("anchor "):
                    name, _, value = line[len("anchor ") :].partition("=")
                    x, y = value.split(",")
                    found[name] = (int(x), int(y))
            if found:
                return found
            time.sleep(0.05)
        raise RuntimeError("the probe printed no anchors")

    def point(self, logical: tuple[float, float]) -> tuple[int, int]:
        """Translate client-relative logical pixels to absolute physical ones."""
        left, top = client_origin(self.hwnd)
        return (
            left + int(round(logical[0] * self.scale)),
            top + int(round(logical[1] * self.scale)),
        )

    def grab(self, logical: tuple[float, float, float, float], name: str):
        left, top = client_origin(self.hwnd)
        box = (
            left + int(logical[0] * self.scale),
            top + int(logical[1] * self.scale),
            left + int(logical[2] * self.scale),
            top + int(logical[3] * self.scale),
        )
        image = ImageGrab.grab(bbox=box, all_screens=True)
        if name:
            image.save(f"target/overlays-{self.mode}-{name}.png")
        return image

    def close(self) -> None:
        self.process.terminate()
        try:
            self.process.wait(timeout=10)
        except subprocess.TimeoutExpired:
            self.process.kill()


def move_pointer(point: tuple[int, int]) -> None:
    user32.SetCursorPos(point[0], point[1])
    time.sleep(0.05)


def click(point: tuple[int, int]) -> None:
    move_pointer(point)
    user32.mouse_event(MOUSEEVENTF_LEFTDOWN, 0, 0, 0, 0)
    time.sleep(0.03)
    user32.mouse_event(MOUSEEVENTF_LEFTUP, 0, 0, 0, 0)
    time.sleep(0.35)


def hover(point: tuple[int, int]) -> None:
    move_pointer(point)


def wheel(point: tuple[int, int], notches: int) -> None:
    move_pointer(point)
    for _ in range(abs(notches)):
        user32.mouse_event(MOUSEEVENTF_WHEEL, 0, 0, -WHEEL_DELTA if notches > 0 else WHEEL_DELTA, 0)
        time.sleep(0.05)
    time.sleep(0.35)


def escape() -> None:
    user32.keybd_event(VK_ESCAPE, 0, 0, 0)
    time.sleep(0.04)
    user32.keybd_event(VK_ESCAPE, 0, KEYEVENTF_KEYUP, 0)
    time.sleep(0.3)


def brightness(pixel: tuple[int, int, int]) -> float:
    return (pixel[0] + pixel[1] + pixel[2]) / 3.0


def first_bright_row(image, x: int, start: int, stop: int, limit: float = 150.0) -> int | None:
    """First row at or after start whose pixel at x is brighter than the page."""
    for y in range(max(start, 0), min(stop, image.height)):
        if brightness(image.getpixel((x, y))[:3]) > limit:
            return y
    return None


def bright_columns(image, y: int, start: int, stop: int, limit: float = 150.0) -> list[int]:
    return [
        x
        for x in range(max(start, 0), min(stop, image.width))
        if brightness(image.getpixel((x, y))[:3]) > limit
    ]


def light_surface(image, limit: float = 150.0, least: int = 400):
    """Bounding box and area of the bright pixels, or None if there is no surface.

    `least` filters out stray highlights: a tooltip or a panel covers far more
    pixels than any glyph or border on the dark page.
    """
    left = top = None
    right = bottom = 0
    count = 0
    for y in range(image.height):
        for x in range(image.width):
            if brightness(image.getpixel((x, y))[:3]) > limit:
                count += 1
                left = x if left is None else min(left, x)
                top = y if top is None else min(top, y)
                right = max(right, x)
                bottom = max(bottom, y)
    if count < least:
        return None
    return {"box": (left, top, right, bottom), "area": count}


POPOVER_BOX = (500.0, 90.0, 860.0, 500.0)


def panel_top(probe: Probe, image, box, x_logical: float, from_logical: float) -> float | None:
    """Top edge of the first bright surface below `from_logical`, in logical px."""
    x = int((x_logical - box[0]) * probe.scale)
    start = int((from_logical - box[1]) * probe.scale)
    row = first_bright_row(image, x, start, image.height)
    return None if row is None else box[1] + row / probe.scale


def bright_runs(probe: Probe, x_logical: float, y0: float, y1: float, limit: float = 150.0):
    """Bright bands down one column, in logical coordinates.

    Reading a column beats a bounding box here: the trigger and the panel are two
    separate bands with a gap between them, and the gap is what the check wants.
    """
    image = probe.grab((x_logical - 2.0, y0, x_logical + 2.0, y1), "")
    column = image.width // 2
    runs: list[tuple[float, float]] = []
    start = None
    for y in range(image.height):
        lit = brightness(image.getpixel((column, y))[:3]) > limit
        if lit and start is None:
            start = y
        elif not lit and start is not None:
            runs.append((y0 + start / probe.scale, y0 + y / probe.scale))
            start = None
    if start is not None:
        runs.append((y0 + start / probe.scale, y1))
    return runs


def check_popover(probe: Probe, results: dict, failures: list[str]) -> None:
    anchors = probe.anchors()
    trigger = anchors["trigger"]

    click(probe.point(trigger))
    opened = probe.reader.wait_for("state popover open=true", timeout=3)
    runs = bright_runs(probe, trigger[0], trigger[1] - 24.0, trigger[1] + 170.0)
    results["popover_opened"] = {"log": opened, "runs": [(round(a, 1), round(b, 1)) for a, b in runs]}
    print(f"popover open: log={opened!r} bright bands={runs}")
    if not opened or len(runs) < 2:
        failures.append("clicking the trigger did not open a visible panel")
        return

    button_bottom = runs[0][1]
    panel_top = runs[1][0]
    gap = panel_top - button_bottom
    results["popover_gap"] = round(gap, 1)
    print(f"gap between the trigger ({button_bottom:.1f}) and the panel ({panel_top:.1f}): {gap:.1f} logical px")
    if not 2.0 <= gap <= 6.0:
        failures.append(f"the panel sits {gap:.1f}px below the trigger instead of the 4px anchor gap")

    # A press on a control inside the panel belongs to that control.
    inside = (trigger[0] + 10.0, (runs[-1][0] + runs[-1][1]) / 2.0)
    click(probe.point(inside))
    child = probe.reader.wait_for("event child=clicked", timeout=2)
    closed = probe.reader.count("state popover open=false")
    results["popover_child_press"] = {"log": child, "closed_by_inside_press": closed > 0}
    print(f"press inside the panel at {inside}: child={child!r} closed={closed > 0}")
    if child is None:
        failures.append("a press on the panel's own button never reached it")
    if closed:
        failures.append("a press inside the panel closed it")

    # Escape closes it while the keyboard is inside.
    escape()
    closed = probe.reader.wait_for("state popover open=false", timeout=2)
    results["popover_escape"] = {"log": closed}
    print(f"escape: {closed!r}")
    if closed is None:
        failures.append("escape did not close the popover")

    # A press outside closes it, and the panel disappears.
    click(probe.point(trigger))
    probe.reader.wait_for("state popover open=true", timeout=3)
    click(probe.point((150, 500)))
    outside = probe.reader.wait_for("state popover open=false", timeout=2)
    after = bright_runs(probe, trigger[0], trigger[1] - 24.0, trigger[1] + 170.0)
    gone = len(after) < 2
    results["popover_outside_press"] = {"log": outside, "panel_gone": gone, "runs": [(round(a, 1), round(b, 1)) for a, b in after]}
    print(f"press outside: {outside!r} bands now={after} panel_gone={gone}")
    if outside is None:
        failures.append("a press outside did not close the popover")
    if not gone:
        failures.append("the panel was still painted after it closed")


def tooltip_around(probe: Probe, x0: float, x1: float):
    """Look for a tooltip surface above and below the target band."""
    for y0, y1 in ((48.0, 114.0), (162.0, 268.0)):
        image = probe.grab((x0, y0, x1, y1), "")
        found = light_surface(image)
        if found:
            left, top, right, bottom = found["box"]
            return {
                "band": (y0, y1),
                "box": (
                    round(x0 + left / probe.scale, 1),
                    round(y0 + top / probe.scale, 1),
                    round(x0 + right / probe.scale, 1),
                    round(y0 + bottom / probe.scale, 1),
                ),
                "area": found["area"],
            }
    return None


def wait_for_tooltip(probe: Probe, x0: float, x1: float, appear: bool, timeout: float):
    start = time.time()
    while time.time() - start < timeout:
        found = tooltip_around(probe, x0, x1)
        if (found is not None) == appear:
            return time.time() - start, found
        time.sleep(0.05)
    return -1.0, tooltip_around(probe, x0, x1)


def check_tooltip(probe: Probe, results: dict, failures: list[str]) -> None:
    anchors = probe.anchors()

    hover(probe.point(anchors["left"]))
    latency, found = wait_for_tooltip(probe, 60.0, 250.0, True, 3.0)
    results["tooltip_left"] = {"latency": round(latency, 2), "surface": found}
    print(f"hover left: tooltip after {latency:.2f}s {found}")
    if found is None:
        failures.append("hovering the left button never showed a tooltip")
    probe.grab((60, 48, 250, 268), "tooltip-left")

    hover(probe.point((450, 560)))
    latency_off, still = wait_for_tooltip(probe, 60.0, 250.0, False, 3.0)
    results["tooltip_hidden"] = {"latency": round(latency_off, 2), "still_there": still}
    print(f"move away: hidden after {latency_off:.2f}s (still={still})")
    if latency_off < 0:
        failures.append("the tooltip stayed after the pointer left")

    # Near the right edge the surface has to stay inside the window.
    hover(probe.point(anchors["right"]))
    latency, found = wait_for_tooltip(probe, 660.0, probe.width, True, 3.0)
    results["tooltip_right"] = {"latency": round(latency, 2), "surface": found}
    print(f"hover right: tooltip after {latency:.2f}s {found}")
    if found is None:
        failures.append("hovering the right button never showed a tooltip")
    else:
        right_edge = found["box"][2]
        results["tooltip_right_edge"] = round(right_edge, 1)
        print(f"tooltip right edge: {right_edge:.1f} logical px of {probe.width}")
        if right_edge > probe.width - 2:
            failures.append("the tooltip ran past the window edge")


MENU_BOX = (280.0, 100.0, 880.0, 560.0)


def menu_surfaces(probe: Probe):
    """The menu panel's bounding box and how far its right edge reaches."""
    image = probe.grab(MENU_BOX, "menu")
    found = light_surface(image, least=2000)
    if found is None:
        return None, image, 0
    left, top, right, bottom = found["box"]
    box = (
        round(MENU_BOX[0] + left / probe.scale, 1),
        round(MENU_BOX[1] + top / probe.scale, 1),
        round(MENU_BOX[0] + right / probe.scale, 1),
        round(MENU_BOX[1] + bottom / probe.scale, 1),
    )
    return box, image, found["area"]


def bright_beyond(probe: Probe, box, y_logical: float, from_logical: float) -> int:
    """Count bright pixels on one row to the right of the main panel."""
    image = probe.grab(box, "")
    y = int((y_logical - box[1]) * probe.scale)
    if not 0 <= y < image.height:
        return 0
    start = int((from_logical - box[0]) * probe.scale)
    return len(bright_columns(image, y, start, image.width))


def check_menu(probe: Probe, results: dict, failures: list[str]) -> None:
    anchors = probe.anchors()

    click(probe.point(anchors["trigger"]))
    opened = probe.reader.wait_for("state menu open=true", timeout=3)
    panel, _, area = menu_surfaces(probe)
    results["menu_opened"] = {"log": opened, "panel": panel, "area": area}
    print(f"menu open: log={opened!r} panel={panel} area={area}")
    if not opened or panel is None:
        failures.append("clicking the menu trigger did not open a visible menu")
        return

    # Rows are 32 tall with a 2px gap inside a 4px padding: the fourth row's
    # centre sits 4 + 3 * 34 + 16 below the panel's top edge. The measured box
    # above also contains the trigger, so the panel top comes from the trigger.
    panel_top = anchors["trigger"][1] + 20.0
    row4 = panel_top + 4.0 + 3.0 * 34.0 + 16.0

    # A quick pass over the submenu row must not open anything.
    hover(probe.point((anchors["row4"][0], row4)))
    time.sleep(0.12)
    quick = bright_beyond(probe, MENU_BOX, row4, 520.0)
    results["submenu_quick_pass"] = {"bright_pixels": quick, "row_y": round(row4, 1)}
    print(f"quick pass over the submenu row ({row4:.0f}): {quick} bright pixels beyond the panel")
    if quick > 10:
        failures.append("a 120ms pass over the submenu row already opened a submenu")

    # Staying on the row past the delay must open it.
    time.sleep(0.45)
    settled = bright_beyond(probe, MENU_BOX, row4, 520.0)
    after, _, _ = menu_surfaces(probe)
    results["submenu_after_delay"] = {"bright_pixels": settled, "panel": after}
    print(f"after the hover delay: {settled} bright pixels beyond the panel, panel={after}")
    if settled <= 10:
        failures.append("the submenu never appeared after the hover delay")

    # Escape unwinds one level, then the whole menu.
    escape()
    time.sleep(0.25)
    first = bright_beyond(probe, MENU_BOX, row4, 520.0)
    results["submenu_escape"] = {"bright_pixels": first}
    print(f"after escape: {first} bright pixels in the submenu row")
    if first > 10:
        failures.append("escape did not close the submenu")

    escape()
    closed = probe.reader.wait_for("state menu open=false", timeout=2)
    results["menu_escape"] = {"log": closed}
    print(f"escape again: {closed!r}")
    if closed is None:
        failures.append("escape did not close the menu chain")


SCROLL_BOX = (60.0, 80.0, 460.0, 420.0)


def check_scroll(probe: Probe, results: dict, failures: list[str]) -> None:
    anchors = probe.anchors()
    trigger = anchors["trigger"]

    click(probe.point(trigger))
    opened = probe.reader.wait_for("state popover open=true", timeout=3)
    before = probe.grab(SCROLL_BOX, "scroll-before")
    top_before = panel_top(probe, before, SCROLL_BOX, trigger[0] + 20.0, trigger[1] + 22.0)
    results["scroll_open"] = {"log": opened, "panel_top": top_before}
    print(f"scrolled popover open: log={opened!r} panel_top={top_before}")
    if opened is None or top_before is None:
        failures.append("the scrolled trigger did not open a panel")
        return

    # Wheel over the container but below the panel, so the container scrolls and
    # the panel is not the thing that takes the wheel.
    wheel(probe.point((200.0, 320.0)), 1)
    after = probe.grab(SCROLL_BOX, "scroll-after")
    top_after = panel_top(probe, after, SCROLL_BOX, trigger[0] + 20.0, 0.0)
    moved = None if top_after is None else top_before - top_after
    results["scroll_follow"] = {"panel_top_before": top_before, "panel_top_after": top_after, "moved_logical": moved}
    print(f"after the wheel: panel_top={top_after} moved={moved}")
    if moved is None:
        failures.append("the panel disappeared while the trigger scrolled")
    elif not 20.0 <= moved <= 160.0:
        failures.append(f"the panel moved {moved:.1f}px, which does not match the scrolled trigger")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--exe", default="target/debug/examples/overlays_probe.exe")
    parser.add_argument("--only", default="")
    args = parser.parse_args()

    repo = Path(__file__).resolve().parent.parent
    exe = (repo / args.exe).resolve()
    if not exe.exists():
        print(f"missing {exe}; build it with `cargo build -p fluent-gallery --example overlays_probe`")
        return 2

    enable_dpi_awareness()
    results: dict = {}
    failures: list[str] = []
    checks = {
        "popover": check_popover,
        "tooltip": check_tooltip,
        "menu": check_menu,
        "scroll": check_scroll,
    }
    modes = [args.only] if args.only else list(checks)

    for mode in modes:
        probe = Probe(exe, mode)
        try:
            results[mode] = {"scale": probe.scale, "client": list(client_size(probe.hwnd))}
            print(f"--- {mode} (scale {probe.scale}) ---")
            checks[mode](probe, results, failures)
        finally:
            probe.close()

    results["failures"] = failures
    report = repo / "target" / "overlays-native.json"
    report.parent.mkdir(parents=True, exist_ok=True)
    report.write_text(json.dumps(results, indent=2), encoding="utf-8")
    print(f"report written to {report}")
    print(json.dumps({"failures": failures}, indent=2))
    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main())
