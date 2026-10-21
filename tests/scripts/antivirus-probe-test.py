#!/usr/bin/env python3
"""Probe options preserve defaults without generating antivirus test content."""
import os
from pathlib import Path
import subprocess
import tempfile
import unittest

PROJECT = Path(__file__).resolve().parents[2]
SOURCE = Path(os.environ.get('ELY_LO_SRC', PROJECT / 'engines/lo-src'))


@unittest.skipUnless((SOURCE / 'configure.ac').is_file(), 'no engine source')
class ProbeOptionTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        text = (SOURCE / 'configure.ac').read_text()
        start = text.index('AC_ARG_ENABLE([antivirus-check],')
        end = text.index('    AC_MSG_CHECKING([for active Antivirus software])', start)
        block = text[start:end]
        cls.temp = tempfile.TemporaryDirectory()
        cls.addClassCleanup(cls.temp.cleanup)
        cls.root = Path(cls.temp.name)
        fixture = 'AC_INIT([probe-options], [1])\n_os=WINNT\nSRC_ROOT=.\n' + block
        fixture += '    AC_MSG_NOTICE([TEST_PROBE_BODY])\nfi\nAC_OUTPUT\n'
        (cls.root / 'configure.ac').write_text(fixture)
        (cls.root / 'antivirusDetection.vbs').write_text('')
        subprocess.run(['autoconf'], cwd=cls.root, check=True, capture_output=True, text=True)

    def configure(self, *args):
        return subprocess.run(['bash', './configure', *args], cwd=self.root,
                              env=dict(os.environ, CONFIG_SITE='/dev/null'),
                              capture_output=True, text=True)

    def test_default_runs_probe(self):
        result = self.configure()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn('TEST_PROBE_BODY', result.stdout)
        self.assertNotIn('explicitly skipped', result.stdout)

    def test_explicit_enable_runs_probe(self):
        result = self.configure('--enable-antivirus-check=yes')
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn('TEST_PROBE_BODY', result.stdout)

    def test_explicit_disable_skips_probe(self):
        result = self.configure('--disable-antivirus-check')
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn('explicitly skipped; system protection is unchanged', result.stdout)
        self.assertNotIn('TEST_PROBE_BODY', result.stdout)

    def test_invalid_value_is_rejected(self):
        result = self.configure('--enable-antivirus-check=maybe')
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('accepts only yes or no', result.stderr)
        self.assertNotIn('TEST_PROBE_BODY', result.stdout)


if __name__ == '__main__':
    unittest.main(verbosity=2)
