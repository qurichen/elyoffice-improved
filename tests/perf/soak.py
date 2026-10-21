#!/usr/bin/env python3
"""Long editing and switching in ElyDoc, checked as it goes."""

import argparse
import os
import shutil
import sys
import tempfile
import time
import zipfile
import xml.etree.ElementTree as ET

import bench

W = "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}"


def words(path):
    """The saved text, read without LibreOffice; a broken file raises."""
    with zipfile.ZipFile(path) as archive:
        body = ET.fromstring(archive.read("word/document.xml"))
    # Runs join without spaces; a word may span two.
    paragraphs = ("".join(node.text or "" for node in p.iter(W + "t")) for p in body.iter(W + "p"))
    return " ".join(paragraphs).split()


def check(paths, n):
    """What the saved files get wrong after `n` cycles."""
    a, b = words(paths["a.docx"]), words(paths["b.docx"])
    problems = [f"a lacks A{k}" for k in range(1, n + 1) if f"A{k}" not in a]
    problems += [f"b keeps B{k}" for k in range(1, n + 1) if f"B{k}" in b]
    if any(word.startswith("A") and word[1:].isdigit() for word in b):
        problems.append("b holds a's text")
    return problems


def saves(app):
    """The app's last save lines, for a failure."""
    return [line[-90:] for line in app.lines() if "save" in line][-6:]


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--minutes", type=float, default=480)
    args = parser.parse_args()
    if sys.platform != "darwin":
        sys.exit("SKIP: the soak drives macOS windows")
    work = tempfile.mkdtemp()
    bench.tools(work)
    fixture = os.path.join(bench.ROOT, "tests", "fixtures", "office", "t.docx")
    paths = {name: os.path.join(work, name) for name in ["a.docx", "b.docx"]}
    for path in paths.values():
        shutil.copy(fixture, path)
    app = bench.App("ely-doc", os.path.join(work, "home"), list(paths.values()))
    def save():
        """Saves through the menu: a keystroke can be lost."""
        bench.sh("osascript", "-e", f'tell application "System Events" to tell (first process whose unix id is {app.process.pid}) '
                 'to click menu item "Save" of menu "File" of menu bar 1')

    def press(title):
        """Chooses a tab, then puts the keyboard in its document."""
        bench.sh(os.path.join(work, "ax-press"), str(app.process.pid), title, "AXRadioButton")
        time.sleep(0.5)
        area = next(line for line in app.ax().splitlines() if line.startswith("AXTextArea"))
        x, y = area.rsplit("at=", 1)[1].split(",")
        bench.sh(os.path.join(work, "click"), x, y)
        # The click may land mid-word; typing goes at the end.
        bench.keys("key code 119 using command down")
        time.sleep(0.3)

    try:
        app.wait("opened b.docx")
        app.front()
        end, n, memory = time.monotonic() + args.minutes * 60, 0, []
        while time.monotonic() < end:
            n += 1
            press("a.docx")
            bench.keys(f'keystroke " A{n} "')
            save()
            press("b.docx")
            bench.keys(f'keystroke "B{n}"')
            time.sleep(0.3)
            bench.keys('keystroke "z" using command down')
            save()
            time.sleep(0.5)
            if n % 10 == 0:
                # Saves finish in their own time; give them ten seconds.
                for _ in range(40):
                    problems = check(paths, n)
                    if not problems:
                        break
                    time.sleep(0.25)
                if problems:
                    raise RuntimeError(f"cycle {n}: {problems[:5]}; a ends {words(paths['a.docx'])[-8:]}; log: {saves(app)}")
                memory.append(app.memory_mb())
                print(f"cycle {n}: ok, {memory[-1]:.0f} MB", flush=True)
    finally:
        app.stop()
        shutil.rmtree(work, ignore_errors=True)
    print(f"ok: {n} cycles in {args.minutes} min; memory MB {memory[0]:.0f} → {memory[-1]:.0f}")


if __name__ == "__main__":
    main()
