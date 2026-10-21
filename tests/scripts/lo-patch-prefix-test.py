#!/usr/bin/env python3
"""Patch detection accepts valid prefixes and rejects unrelated edits."""
import os
from pathlib import Path
import subprocess
import tempfile
import unittest

PROJECT = Path(__file__).resolve().parents[2]


class PrefixTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        (self.root / 'scripts').mkdir()
        self.script = self.root / 'scripts/apply-lo-patches.sh'
        self.script.write_text((PROJECT / 'scripts/apply-lo-patches.sh').read_text())
        self.patches = self.root / 'patches/libreoffice'
        self.patches.mkdir(parents=True)
        self.tree = self.root / 'tree'
        self.tree.mkdir()
        self.git('init', '-q')
        self.git('config', 'core.autocrlf', 'false')
        self.git('config', 'user.name', 'fixture')
        self.git('config', 'user.email', 'fixture@example.invalid')
        self.git('config', 'commit.gpgsign', 'false')
        (self.root / 'no-hooks').mkdir()
        self.git('config', 'core.hooksPath', str(self.root / 'no-hooks'))
        (self.tree / 'source.txt').write_text('base\n')
        self.commit()
        self.base = self.git('rev-parse', 'HEAD').stdout.strip()
        for number, content in enumerate(('one\n', 'two\n', 'three\n'), 1):
            (self.tree / 'source.txt').write_text(content)
            if number >= 2:
                (self.tree / 'added.txt').write_text(content)
            self.git('add', '-A')
            patch = self.git('diff', '--cached', '--binary', 'HEAD').stdout
            (self.patches / f'{number:04d}-fixture.patch').write_text(patch)
            self.commit()
        self.git('checkout', '-q', '--detach', self.base)

    def git(self, *args):
        return subprocess.run(['git', '-C', str(self.tree), *args], capture_output=True,
                              text=True, check=True)

    def commit(self):
        self.git('add', '-A')
        self.git('commit', '-qm', 'fixture')

    def apply(self):
        return subprocess.run(['bash', str(self.script), str(self.tree)],
                              capture_output=True, text=True, env=dict(os.environ))

    def assert_complete(self, result, prefix):
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn(f'{prefix} of 3 patches already', result.stdout)
        self.assertEqual((self.tree / 'source.txt').read_text(), 'three\n')
        self.assertEqual((self.tree / 'added.txt').read_text(), 'three\n')

    def test_pristine_and_repeat(self):
        self.assert_complete(self.apply(), 0)
        result = self.apply()
        self.assert_complete(result, 3)
        self.assertNotIn('applied 000', result.stdout)

    def test_middle_prefix(self):
        for patch in sorted(self.patches.glob('*.patch'))[:2]:
            self.git('apply', str(patch))
        self.assert_complete(self.apply(), 2)

    def test_unrelated_edit_is_rejected(self):
        (self.tree / 'source.txt').write_text('unrelated\n')
        result = self.apply()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('matches no prefix', result.stderr)
        self.assertEqual((self.tree / 'source.txt').read_text(), 'unrelated\n')
        self.assertFalse((self.tree / 'added.txt').exists())


if __name__ == '__main__':
    unittest.main(verbosity=2)
