#!/usr/bin/env python3
"""Keep XMLSec's Windows source encoding explicit and warnings fatal."""
import os
from pathlib import Path
import re
import shutil
import subprocess
import tempfile
import unittest

PROJECT = Path(__file__).resolve().parents[2]
SOURCE = Path(os.environ.get('ELY_LO_SRC', PROJECT / 'engines/lo-src'))
RECIPE = SOURCE / 'external/xmlsec/ExternalProject_xmlsec.mk'


@unittest.skipUnless(RECIPE.is_file(), 'no engine source')
class XmlsecEncodingTests(unittest.TestCase):
    def test_windows_recipe_selects_utf8(self):
        text = RECIPE.read_text(encoding='utf-8')
        windows = text.split('ifeq ($(OS),WNT)\n', 1)[1].split('\nelse\n', 1)[0]
        flags = re.search(r'cflags="([^"]+)"', windows).group(1).split()
        self.assertIn('/utf-8', flags)
        self.assertIn('$(SOLARINC)', flags)
        self.assertNotIn('/wd4819', flags)
        self.assertNotIn('/WX-', flags)
        self.assertIn('&& nmake', windows)

    @unittest.skipUnless(os.name == 'nt' and shutil.which('cl.exe'),
                         'native Windows developer environment required')
    def test_real_msvc_encoding_failure_and_fix(self):
        local = PROJECT / '.work/xmlsec-encoding-tests'
        local.mkdir(parents=True, exist_ok=True)
        with tempfile.TemporaryDirectory(dir=local) as folder:
            root = Path(folder)
            (root / 'unicode.h').write_text('// €\n', encoding='utf-8')
            source = root / 'probe.cpp'
            source.write_text('#include "unicode.h"\nconst char text[] = "€";\n'
                              'int main() { return sizeof(text) == 4 && '
                              '(unsigned char)text[0] == 0xe2 ? 0 : 1; }\n', encoding='utf-8')
            before = subprocess.run(['cl.exe', '/nologo', '/WX', '/c',
                                     '/source-charset:.936', str(source)], cwd=root,
                                    capture_output=True)
            self.assertNotEqual(before.returncode, 0)
            self.assertIn(b'C4819', before.stdout + before.stderr)
            after = subprocess.run(['cl.exe', '/nologo', '/WX', '/utf-8',
                                    str(source), '/Fe:probe.exe'], cwd=root,
                                   capture_output=True)
            self.assertEqual(after.returncode, 0, repr(after.stdout + after.stderr))
            subprocess.run([str(root / 'probe.exe')], check=True)


if __name__ == '__main__':
    unittest.main(verbosity=2)
