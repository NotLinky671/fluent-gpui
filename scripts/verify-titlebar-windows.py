"""Exercise only windows created by the titlebar_probe acceptance fixture.

Build: cargo build -p fluent-gallery --example titlebar_probe --locked
Run: python scripts/verify-titlebar-windows.py
Optional Pillow adds screenshots; assertions require only Python's stdlib.
WM_NCHITTEST return codes: https://learn.microsoft.com/en-us/windows/win32/inputdev/wm-nchittest
This sends real window messages, not physical pointer/drag or Snap gestures.
"""
import ctypes as c
from ctypes import wintypes as w
import json
import os
from pathlib import Path
import subprocess
import time

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / "target" / "titlebar-native"
OUT.mkdir(parents=True, exist_ok=True)
EXE = ROOT / "target" / "debug" / "examples" / "titlebar_probe.exe"
u = c.WinDLL("user32", use_last_error=True)
u.SetProcessDPIAware()
u.SendMessageTimeoutW.argtypes = [w.HWND, w.UINT, w.WPARAM, w.LPARAM, w.UINT, w.UINT, c.POINTER(c.c_size_t)]
u.SendMessageTimeoutW.restype = c.c_ssize_t
u.GetWindowThreadProcessId.argtypes = [w.HWND, c.POINTER(w.DWORD)]
u.GetClientRect.argtypes = [w.HWND, c.POINTER(w.RECT)]
u.ClientToScreen.argtypes = [w.HWND, c.POINTER(w.POINT)]
u.GetWindowRect.argtypes = [w.HWND, c.POINTER(w.RECT)]
u.GetDpiForWindow.argtypes = [w.HWND]
u.GetDpiForWindow.restype = w.UINT
u.IsZoomed.argtypes = [w.HWND]
u.IsIconic.argtypes = [w.HWND]
u.ShowWindow.argtypes = [w.HWND, c.c_int]
u.IsWindowVisible.argtypes = [w.HWND]
u.SetForegroundWindow.argtypes = [w.HWND]
u.GetWindowTextW.argtypes = [w.HWND, w.LPWSTR, c.c_int]
CALLBACK = c.WINFUNCTYPE(w.BOOL, w.HWND, w.LPARAM)
u.EnumWindows.argtypes = [CALLBACK, w.LPARAM]


def send(hwnd, message, wp, lp):
    result = c.c_size_t()
    if not u.SendMessageTimeoutW(hwnd, message, wp, lp, 0x3, 2000, c.byref(result)):
        raise TimeoutError(f"fixture message {message:#x} failed or exceeded 2s")
    return c.c_ssize_t(result.value).value


def wait_for(check, label, seconds=8):
    deadline = time.monotonic() + seconds
    while time.monotonic() < deadline:
        value = check()
        if value:
            return value
        time.sleep(0.03)
    raise AssertionError("timed out: " + label)


def windows(pid):
    found = []
    @CALLBACK
    def visit(hwnd, _):
        owner = w.DWORD()
        u.GetWindowThreadProcessId(hwnd, c.byref(owner))
        title = c.create_unicode_buffer(256)
        u.GetWindowTextW(hwnd, title, 256)
        if owner.value == pid and title.value.startswith("Fluent Titlebar Probe") and u.IsWindowVisible(hwnd):
            found.append(hwnd)
        return True
    u.EnumWindows(visit, 0)
    return found


def packed(x, y):
    return (int(x) & 0xffff) | ((int(y) & 0xffff) << 16)


def client(hwnd):
    rect = w.RECT()
    assert u.GetClientRect(hwnd, c.byref(rect))
    return rect.right, rect.bottom


def screen(hwnd, x, y):
    pt = w.POINT(int(x), int(y))
    assert u.ClientToScreen(hwnd, c.byref(pt))
    return pt.x, pt.y


def point_for(hwnd, offset, y=16):
    scale = u.GetDpiForWindow(hwnd) / 96
    width, _ = client(hwnd)
    return round(width - offset * scale), round(y * scale)


def hit(hwnd, x, y):
    # GPUI's hit-test callback reads its last mouse hit set. First deliver the
    # pointer move, as Windows would before a press, then query at that point.
    send(hwnd, 0x200, 0, packed(x, y))
    return send(hwnd, 0x84, 0, packed(*screen(hwnd, x, y)))


def caption_click(hwnd, offset, expected):
    wait_for(lambda: hit(hwnd, *point_for(hwnd, offset)) == expected, f"caption frame after native state change ({offset}, expected {expected})")
    x, y = point_for(hwnd, offset)
    pos = packed(*screen(hwnd, x, y))
    send(hwnd, 0xA1, expected, pos)
    send(hwnd, 0xA2, expected, pos)


def screenshot(hwnd, name):
    try:
        from PIL import ImageGrab
    except ImportError:
        return
    u.SetForegroundWindow(hwnd)
    # A bounded render allowance is for the visual sample only. Behaviour
    # assertions below wait on IsZoomed/IsIconic/process/window/log state.
    time.sleep(0.2)
    width, height = client(hwnd)
    left, top = screen(hwnd, 0, 0)
    # PrintWindow returns a black bitmap for this GPU surface; capture only the
    # fixture's client rectangle from the desktop compositor instead.
    u.GetForegroundWindow.restype = w.HWND
    if u.GetForegroundWindow() != hwnd:
        results.setdefault("screenshots", {})[name] = "skipped: fixture not foreground"
        return
    before = w.POINT()
    u.GetCursorPos(c.byref(before))
    u.SetCursorPos(*screen(hwnd, *point_for(hwnd, 23)))
    try:
        time.sleep(0.2)
        image = ImageGrab.grab(bbox=(left, top, left + width, top + height), all_screens=True)
    finally:
        u.SetCursorPos(before.x, before.y)
    image.save(OUT / (name + ".png"))
    results.setdefault("screenshots", {})[name] = "captured" if image.getbbox() else "invalid: black capture"


def run(mode, test):
    path = OUT / (mode + ".log")
    env = os.environ.copy()
    for key in list(env):
        if key.startswith("FLUENT_GALLERY_"):
            del env[key]
    with path.open("w", encoding="utf-8") as log:
        process = subprocess.Popen([str(EXE), mode], cwd=ROOT, env=env, stdout=log, stderr=log)
        try:
            count = 2 if mode == "two" else 1
            handles = wait_for(lambda: (v if len(v := windows(process.pid)) == count else None), "fixture windows")
            for hwnd in handles:
                wait_for(lambda: hit(hwnd, *point_for(hwnd, 23)) == 20, "first caption frame")
            test(process, handles, path)
        finally:
            if process.poll() is None:
                for hwnd in windows(process.pid):
                    # Fixture cleanup only; never target any other application's windows.
                    try:
                        send(hwnd, 0x10, 0, 0)
                        send(hwnd, 0x10, 0, 0)  # The fixture may veto once.
                    except TimeoutError:
                        pass  # The bounded process cleanup below still runs.
                try:
                    process.wait(timeout=3)
                except subprocess.TimeoutExpired:
                    process.terminate()
                    process.wait(timeout=3)


results = {}


def basic(process, handles, path):
    hwnd = handles[0]
    scale = u.GetDpiForWindow(hwnd) / 96
    checks = {}
    for label, x, y in [("left-padding", 3, 16), ("below-title", 14, 28), ("empty-strip", 280, 16)]:
        value = hit(hwnd, round(x * scale), round(y * scale))
        assert value == 2, (label, value)
        checks[label] = value
    for label, offset, code in [("minimize", 115, 8), ("maximize", 69, 9), ("close", 23, 20)]:
        value = hit(hwnd, *point_for(hwnd, offset))
        assert value == code, (label, value)
        checks[label] = value
    screenshot(hwnd, "normal-hover-close")
    caption_click(hwnd, 69, 9)
    wait_for(lambda: u.IsZoomed(hwnd), "maximized")
    screenshot(hwnd, "maximized-restore-glyph")
    caption_click(hwnd, 69, 9)
    wait_for(lambda: not u.IsZoomed(hwnd), "restored")
    caption_click(hwnd, 115, 8)
    wait_for(lambda: u.IsIconic(hwnd), "minimized")
    u.ShowWindow(hwnd, 9)
    wait_for(lambda: not u.IsIconic(hwnd), "restored from minimized")
    # Open the fixture's actual Fluent Dialog with client mouse messages.
    pos = packed(round(75 * scale), round(72 * scale))
    send(hwnd, 0x200, 0, pos)
    send(hwnd, 0x201, 1, pos)
    send(hwnd, 0x202, 0, pos)
    wait_for(lambda: "dialog-open" in path.read_text(encoding="utf-8"), "Fluent Dialog opened")
    screenshot(hwnd, "dialog-caption")
    assert hit(hwnd, *point_for(hwnd, 23)) == 20, "modal must not hide caption hit area"
    caption_click(hwnd, 23, 20)
    wait_for(lambda: process.poll() is not None, "native close and last-window exit")
    results["basic"] = {"dpi": round(scale * 96), "hits": checks, "maximize_restore_minimize_close": True}


def veto(process, handles, path):
    hwnd = handles[0]
    caption_click(hwnd, 23, 20)
    wait_for(lambda: "should-close:0:false" in path.read_text(encoding="utf-8"), "close guard")
    assert process.poll() is None and len(windows(process.pid)) == 1
    caption_click(hwnd, 23, 20)
    wait_for(lambda: process.poll() is not None, "accepted close")
    results["veto"] = {"rejected_window_preserved": True, "accepted_close_exited": True}


def two(process, handles, _):
    caption_click(handles[0], 23, 20)
    wait_for(lambda: len(windows(process.pid)) == 1, "one window removed")
    assert process.poll() is None
    caption_click(windows(process.pid)[0], 23, 20)
    wait_for(lambda: process.poll() is not None, "last window closes")
    results["two_windows"] = {"closing_one_keeps_other": True, "last_close_exits": True}


try:
    run("basic", basic)
    run("veto", veto)
    run("two", two)
finally:
    (OUT / "results.json").write_text(json.dumps(results, indent=2), encoding="utf-8")
print(json.dumps(results, indent=2))
