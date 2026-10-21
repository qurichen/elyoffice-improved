#!/usr/bin/env python3
"""Build concurrency is explicit and validated before side effects."""
import os
from pathlib import Path
import subprocess
import tempfile
import unittest

PROJECT = Path(__file__).resolve().parents[2]


class BuildJobsTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        (self.root / 'scripts').mkdir()
        (self.root / 'engines/lo-src/.git').mkdir(parents=True)
        source = PROJECT / 'scripts/build-libreoffice.sh'
        (self.root / 'scripts/build-libreoffice.sh').write_text(source.read_text())
        (self.root / 'scripts/fetch-references.sh').write_text('fetch libreoffice-core unused pin\n')
        (self.root / 'scripts/apply-lo-patches.sh').write_text('#!/usr/bin/env bash\nexit 0\n')
        (self.root / 'scripts/apply-lo-patches.sh').chmod(0o755)
        (self.root / 'engines/lo-src/autogen.sh').write_text(
            '#!/usr/bin/env bash\nprintf "%s\\n" "$@" > "$TEST_AUTOGEN_ARGS"\nexit 0\n')
        (self.root / 'engines/lo-src/autogen.sh').chmod(0o755)
        self.mock = self.root / 'mock'
        self.mock.mkdir()
        for name, body in {
            'uname': 'printf "Linux\\n"',
            'nproc': 'printf "12\\n"',
            'git': 'if [[ "$*" == *"rev-parse HEAD" ]]; then printf "pin\\n"; fi',
            'make': 'printf "%s\\n" "$@" > "$TEST_MAKE_ARGS"; exit 77',
        }.items():
            path = self.mock / name
            path.write_text('#!/usr/bin/env bash\n' + body + '\n')
            path.chmod(0o755)
        self.args = self.root / 'make-args'
        self.config = self.root / 'autogen-args'

    def build(self, value=None, arguments=()):
        env = dict(os.environ, PATH=str(self.mock) + os.pathsep + os.environ['PATH'],
                   TEST_MAKE_ARGS=str(self.args), TEST_AUTOGEN_ARGS=str(self.config))
        env.pop('ELY_BUILD_JOBS', None)
        if value is not None:
            env['ELY_BUILD_JOBS'] = value
        return subprocess.run(['bash', str(self.root / 'scripts/build-libreoffice.sh'), *arguments],
                              env=env, capture_output=True, text=True)

    def test_default_uses_cpu_count(self):
        result = self.build()
        self.assertEqual(result.returncode, 77, result.stderr)
        self.assertEqual(self.args.read_text(), '-j12\n')
        self.assertNotIn('--with-parallelism=', self.config.read_text())

    def test_explicit_jobs(self):
        for value in ('1', '4', '16'):
            with self.subTest(value=value):
                result = self.build(value)
                self.assertEqual(result.returncode, 77, result.stderr)
                self.assertEqual(self.args.read_text(), '-j' + value + '\n')
                self.assertIn('--with-parallelism=' + value, self.config.read_text().splitlines())

    def test_invalid_jobs_stop_before_make(self):
        for value in ('', '0', '-1', '+2', '01', '1.5', 'all', ' 4', '4 2', '4\n'):
            with self.subTest(value=value):
                result = self.build(value)
                self.assertEqual(result.returncode, 1, result.stderr)
                self.assertIn('positive decimal integer', result.stderr)
                self.assertFalse(self.args.exists())
                self.assertFalse(self.config.exists())

    def test_extra_configure_options(self):
        result = self.build('4', ('--disable-antivirus-check', '--with-parallelism=16'))
        self.assertEqual(result.returncode, 77, result.stderr)
        options = self.config.read_text().splitlines()
        self.assertIn('--disable-antivirus-check', options)
        self.assertEqual(options[-1], '--with-parallelism=4')
        self.assertEqual(self.args.read_text(), '-j4\n')


if __name__ == '__main__':
    unittest.main(verbosity=2)
