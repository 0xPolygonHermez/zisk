"""The builder may exit zero after LLVM rejects a program; fail on its diagnostic."""
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest

SCRIPT = Path(__file__).resolve().parents[1] / 'scripts/build-sbf.sh'


class BuildScriptTests(unittest.TestCase):
    def test_diagnostics_are_checked_without_optional_search_tools(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / 'scripts').mkdir()
            shutil.copyfile(SCRIPT, root / 'scripts/build-sbf.sh')
            builder = root / '.cache/build-tools/bin/cargo-build-sbf'
            builder.parent.mkdir(parents=True)
            commands = root / 'commands'
            commands.mkdir()
            for name in ['dirname', 'mkdir', 'tee', 'grep']:
                (commands / name).symlink_to(shutil.which(name))
            for diagnostic, succeeds in [
                ('Finished release build', True),
                ('Error: Function exceeds stack offset', False),
                ('LLVM ERROR: Branch target out of insn range', False),
            ]:
                with self.subTest(diagnostic=diagnostic):
                    builder.write_text(
                        '#!/bin/bash\n'
                        'if [[ "$1" == --version ]]; then echo "cargo-build-sbf 4.6.0"; '
                        f'else echo "{diagnostic}"; fi\n')
                    builder.chmod(0o755)
                    result = subprocess.run(
                        ['/bin/bash', str(root / 'scripts/build-sbf.sh')],
                        env={**os.environ, 'PATH': str(commands)}, capture_output=True, text=True)
                    self.assertEqual(result.returncode == 0, succeeds, result.stdout + result.stderr)


if __name__ == '__main__':
    unittest.main()
