#!/usr/bin/env python3
# The bench's verdicts on made-up logs and handles.
import collections
import re
import unittest

import bench


class Verdicts(unittest.TestCase):
    def test_a_failed_draw_voids_the_run(self):
        for line in ["lok: tile paint failed: atlas full", "lok: tile (0, 1) failed: kernel",
                     "pdf: drawing a page failed (Other)"]:
            self.assertTrue(re.search(bench.PAINT_FAILED, line), line)
        self.assertFalse(re.search(bench.PAINT_FAILED, "lok: first tile drawn"))

    def test_a_leak_on_the_first_close_fails(self):
        before = collections.Counter({("REG", "/tmp/a"): 1})
        now = before + collections.Counter({("CHR", "/dev/null"): 1})
        with self.assertRaisesRegex(RuntimeError, "leaves handles"):
            bench.handles_kept(before, now, None, 1)

    def test_system_resources_once_then_steady(self):
        before = collections.Counter({("REG", "/tmp/a"): 1})
        font = collections.Counter({("REG", "/System/Library/Fonts/SFNSItalic.ttf"): 1})
        extra = bench.handles_kept(before, before + font, None, 1)
        self.assertEqual(bench.handles_kept(before, before + font, extra, 2), font)
        more = before + font + collections.Counter({("REG", "/System/Library/Fonts/More.ttf"): 1})
        with self.assertRaisesRegex(RuntimeError, "changed the system handles"):
            bench.handles_kept(before, more, extra, 3)


    def test_the_document_count_must_be_logged(self):
        self.assertEqual(bench.documents_open(["x", "lok-worker: 2 documents open", "lok-worker: 1 documents open"]), 1)
        with self.assertRaisesRegex(RuntimeError, "no document count"):
            bench.documents_open(["office: closed Untitled 2"])

    def test_a_cycle_needs_its_own_count(self):
        log = ["lok-worker: 1 documents open", "ely-doc: opened Untitled 2 as tab 1", "office: closed Untitled 2"]
        cursor = 1
        with self.assertRaisesRegex(RuntimeError, "no document count"):
            bench.documents_open(log[cursor:])


if __name__ == "__main__":
    unittest.main()
