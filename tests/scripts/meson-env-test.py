#!/usr/bin/env python3
"""Exercise the patched Meson activation without decoding console output."""
import ast
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tarfile
import tempfile
import types
import unittest
from unittest.mock import patch

PROJECT = Path(__file__).resolve().parents[2]
SOURCE = Path(os.environ.get('ELY_LO_SRC', PROJECT / 'engines/lo-src'))
TARBALL = SOURCE / 'external/tarballs/meson-1.8.3.tar.gz'
PATCH = PROJECT / 'patches/libreoffice/0058-meson-windows-unicode-environment.patch'


@unittest.skipUnless(TARBALL.is_file(), 'no pinned Meson tarball')
class MesonEnvironmentTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        local = PROJECT / '.work/meson-env-tests'
        local.mkdir(parents=True, exist_ok=True)
        cls.temp = tempfile.TemporaryDirectory(dir=local)
        cls.addClassCleanup(cls.temp.cleanup)
        cls.root = Path(cls.temp.name)
        relative = 'mesonbuild/utils/vsenv.py'
        target = cls.root / relative
        target.parent.mkdir(parents=True)
        with tarfile.open(TARBALL) as archive:
            member = archive.extractfile('meson-1.8.3/' + relative)
            target.write_bytes(member.read())
        lines = PATCH.read_text(encoding='utf-8').splitlines(keepends=True)
        start = lines.index('+diff --git a/' + relative + ' b/' + relative + '\n')
        inner = cls.root / 'meson.patch'
        inner.write_text(''.join(line[1:] for line in lines[start:]), encoding='utf-8', newline='\n')
        subprocess.run(['git', 'init', '-q'], cwd=cls.root, check=True)
        subprocess.run(['git', 'config', 'core.autocrlf', 'false'], cwd=cls.root, check=True)
        subprocess.run(['git', 'apply', '--check', str(inner)], cwd=cls.root, check=True)
        subprocess.run(['git', 'apply', str(inner)], cwd=cls.root, check=True)
        tree = ast.parse(target.read_text(encoding='utf-8'))
        selected = [node for node in tree.body if isinstance(node, ast.FunctionDef)
                    and node.name == '_setup_vsenv']
        selected += [node for node in tree.body if isinstance(node, ast.Assign)
                     and any(isinstance(t, ast.Name) and t.id == 'bat_template'
                             for t in node.targets)]
        cls.code = compile(ast.Module(body=selected, type_ignores=[]), str(target), 'exec')

    def setUp(self):
        self.files = tempfile.TemporaryDirectory(dir=self.root)
        self.addCleanup(self.files.cleanup)
        self.folder = Path(self.files.name)
        self.program_files = self.folder / 'Program Files'
        locator = self.program_files / 'Microsoft Visual Studio/Installer/vswhere.exe'
        locator.parent.mkdir(parents=True)
        locator.touch()
        self.vs = self.folder / 'VS'
        self.vcvars = self.vs / 'VC/Auxiliary/Build/vcvars64.bat'
        self.vcvars.parent.mkdir(parents=True)
        self.vcvars.touch()
        self.ns = dict(os=os, subprocess=subprocess, json=json, pathlib=__import__('pathlib'),
                       shutil=shutil, tempfile=tempfile, sys=sys, MesonException=RuntimeError,
                       is_windows=lambda: True, windows_detect_native_arch=lambda: 'x64',
                       mlog=types.SimpleNamespace(log=lambda *args: None))
        exec(self.code, self.ns)
        self.activate = self.ns['_setup_vsenv']
        self.captured = []

    def run_mock(self, payload=None, failure=None):
        environment = {'ProgramFiles(x86)': str(self.program_files),
                       'COMSPEC': r'C:\Windows\System32\cmd.exe',
                       'MESON_FORCE_VSENV_FOR_UNITTEST': '1'}
        vswhere = json.dumps([{'installationPath': str(self.vs),
                              'catalog': {'productDisplayVersion': 'fixture'}}]).encode()

        def launch(command, **kwargs):
            paths = kwargs['env']
            bat = Path(paths['__MESON_VSENV_BATCH'])
            self.captured.append(bat)
            self.assertEqual(kwargs['stdout'], subprocess.DEVNULL)
            self.assertIn('/d /v:off /c ""%__MESON_VSENV_BATCH%""', command)
            text = bat.read_text(encoding='ascii')
            self.assertIn('IF ERRORLEVEL 1 EXIT /B %ERRORLEVEL%', text)
            self.assertIn(' -I -c ', text)
            self.assertIn('SETLOCAL DisableDelayedExpansion', text)
            self.assertNotIn('\nSET\n', text)
            exported = Path(paths['__MESON_VSENV_EXPORT'])
            self.captured.append(exported)
            if failure:
                raise failure
            if payload.startswith('{'):
                values = json.loads(payload)
                values.update({key: value for key, value in paths.items()
                               if key.startswith('__MESON_VSENV_')})
                content = json.dumps(values, ensure_ascii=False)
            else:
                content = payload
            exported.write_text(content, encoding='utf-8')

        with patch.dict(os.environ, environment, clear=True), \
                patch.object(tempfile, 'tempdir', str(self.folder)), \
                patch.object(subprocess, 'check_output', return_value=vswhere), \
                patch.object(subprocess, 'check_call', side_effect=launch):
            result = self.activate(True)
            values = dict(os.environ)
        return result, values

    def assert_cleaned(self):
        self.assertEqual(len(self.captured), 2)
        self.assertTrue(all(not p.exists() for p in self.captured))

    def test_unicode_values_survive(self):
        exported = {'VSINSTALLDIR': str(self.vs), 'UNICODE': '中文-😀-café-Є',
                    'SPECIAL': 'a=b;"quoted"\nnext line'}
        result, values = self.run_mock(json.dumps(exported, ensure_ascii=False))
        self.assertTrue(result)
        for key, value in exported.items():
            self.assertEqual(values[key], value)
        self.assertFalse(any(key.startswith('__MESON_VSENV_') for key in values))
        self.assert_cleaned()

    def test_activation_error_propagates(self):
        with self.assertRaises(subprocess.CalledProcessError):
            self.run_mock(failure=subprocess.CalledProcessError(23, 'vcvars'))
        self.assert_cleaned()

    def test_invalid_export_is_not_ignored(self):
        with self.assertRaises(json.JSONDecodeError):
            self.run_mock('not JSON')
        self.assert_cleaned()

    def test_missing_locator_is_not_ignored(self):
        with patch.dict(os.environ, {'ProgramFiles(x86)': str(self.folder),
                                   'MESON_FORCE_VSENV_FOR_UNITTEST': '1'}, clear=True):
            with self.assertRaisesRegex(RuntimeError, 'Could not find'):
                self.activate(True)

    def test_existing_environment_is_preserved(self):
        with patch.dict(os.environ, {'VSINSTALLDIR': str(self.vs)}, clear=True):
            self.assertFalse(self.activate(True))

    @unittest.skipUnless(os.name == 'nt' and os.environ.get('ELY_TEST_REAL_VSENV') == '1',
                         'opt-in Windows compiler integration')
    def test_real_vs_activation_and_compiler(self):
        original = dict(os.environ)
        self.addCleanup(lambda: (os.environ.clear(), os.environ.update(original)))
        os.environ['MESON_FORCE_VSENV_FOR_UNITTEST'] = '1'
        os.environ['ELY_UNICODE_PROBE'] = '中文-😀-café-Є'
        self.assertTrue(self.activate(True))
        self.assertEqual(os.environ['ELY_UNICODE_PROBE'], '中文-😀-café-Є')
        self.assertTrue(shutil.which('cl.exe'))
        source = self.folder / 'smoke.cpp'
        source.write_text('#include <windows.h>\nint main() { return GetCurrentProcessId() ? 0 : 1; }\n')
        result = subprocess.run(['cl.exe', '/nologo', str(source), '/Fe:smoke.exe'],
                                cwd=self.folder, capture_output=True)
        self.assertEqual(result.returncode, 0, repr(result.stderr))
        subprocess.run([str(self.folder / 'smoke.exe')], check=True)

    @unittest.skipUnless(os.name == 'nt' and os.environ.get('ELY_TEST_REAL_VSENV') == '1',
                         'opt-in Windows command integration')
    def test_real_unicode_and_metacharacter_paths(self):
        odd = self.folder / '中文 目录&%ELY_EXPAND_ME%!'
        vcvars = odd / 'VS/VC/Auxiliary/Build/vcvars64.bat'
        vcvars.parent.mkdir(parents=True)
        vcvars.write_text('@echo off\nset "VSINSTALLDIR=%~dp0"\nexit /b 0\n',
                          encoding='ascii')
        interpreter = odd / 'python.cmd'
        interpreter.write_text('@"%ELY_MESON_TEST_REAL_PYTHON%" %*\n', encoding='ascii')
        vswhere = json.dumps([{'installationPath': str(odd / 'VS'),
                              'catalog': {'productDisplayVersion': 'fixture'}}]).encode()
        with patch.dict(os.environ, {'MESON_FORCE_VSENV_FOR_UNITTEST': '1',
                                    'ELY_MESON_TEST_REAL_PYTHON': sys.executable,
                                    'ELY_EXPAND_ME': 'WRONG_PATH'}), \
                patch.object(subprocess, 'check_output', return_value=vswhere), \
                patch.object(tempfile, 'tempdir', str(odd)), \
                patch.object(sys, 'executable', str(interpreter)):
            self.assertTrue(self.activate(True))
            self.assertEqual(Path(os.environ['VSINSTALLDIR']), vcvars.parent)
            self.assertFalse(any(key.startswith('__MESON_VSENV_') for key in os.environ))
        self.assertFalse(list(odd.glob('tmp*')))

    @unittest.skipUnless(os.name == 'nt' and os.environ.get('ELY_TEST_REAL_VSENV') == '1',
                         'opt-in Windows command integration')
    def test_real_vcvars_error_stops_export(self):
        self.vcvars.write_text('@echo off\nexit /b 23\n', encoding='ascii')
        vswhere = json.dumps([{'installationPath': str(self.vs),
                              'catalog': {'productDisplayVersion': 'fixture'}}]).encode()
        with patch.dict(os.environ, {'MESON_FORCE_VSENV_FOR_UNITTEST': '1'}), \
                patch.object(subprocess, 'check_output', return_value=vswhere), \
                patch.object(tempfile, 'tempdir', str(self.folder)):
            with self.assertRaises(subprocess.CalledProcessError) as error:
                self.activate(True)
            self.assertEqual(error.exception.returncode, 23)
        self.assertFalse(list(self.folder.glob('tmp*')))


if __name__ == '__main__':
    unittest.main(verbosity=2)
