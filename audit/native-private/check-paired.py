"""Exercise installed Trident -> Joy native private proving and verification."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import tempfile

p = argparse.ArgumentParser(description=__doc__)
p.add_argument('--bin', type=Path, required=True)
p.add_argument('--trident-source', type=Path, required=True)
p.add_argument('--output', type=Path, required=True)
a = p.parse_args()
binary = a.bin.resolve()
root = Path(__file__).resolve().parents[3]
work = Path(tempfile.mkdtemp(prefix='native-paired-cli-'))
env = dict(os.environ, PATH=str(binary) + ':/usr/bin:/bin')
report = dict(status='running', commands=[], work=str(work), sources={}, binaries={})
for name, repo in [('joy', root/'joy'), ('zheng', root/'zheng'), ('trident', a.trident_source)]:
    report['sources'][name] = subprocess.check_output(['git', '-C', str(repo), 'rev-parse', 'HEAD'], text=True).strip()
for name in ['joy', 'trident']:
    report['binaries'][name] = hashlib.sha256((binary/name).read_bytes()).hexdigest()

def run(name, args, expected=0):
    command = [str(binary/name), *args]
    r = subprocess.run(command, cwd=work, env=env, text=True, capture_output=True, timeout=120)
    report['commands'].append(dict(command=command, expected_exit=expected,
        exit_code=r.returncode, stdout=r.stdout, stderr=r.stderr))
    a.output.write_text(json.dumps(report, indent=2) + '\n')
    assert r.returncode == expected, report['commands'][-1]

(work/'private.tri').write_text('program product\nfn main()->Field {let a:Field=divine()\n let b:Field=divine()\n a*b}\n')
(work/'witness.json').write_text('{"schema_version":1,"public":[],"secret":["7","13"]}')
try:
    run('trident', ['run', 'private.tri', '--target', 'nox', '--input-file', 'witness.json'])
    run('trident', ['prove', 'private.tri', '--target', 'nox', '--input-file', 'witness.json', '--output', 'pair.zheng'])
    proof = (work/'pair.zheng').read_bytes()
    assert proof.startswith(b'JOYZH001')
    run('trident', ['verify', 'pair.zheng', '--target', 'nox'])
    # Trident proves with release by default; source-bound verification must match.
    run('joy', ['verify', 'private.tri', '--proof', 'pair.zheng', '--profile', 'release', '--claim', '91'])
    run('joy', ['verify', 'private.tri', '--proof', 'pair.zheng', '--profile', 'debug', '--claim', '91'], 1)
    run('joy', ['verify', 'pair.zheng', '--claim', '92'], 1)
    (work/'corrupt.zheng').write_bytes(proof[:-1] + bytes([proof[-1] ^ 1]))
    run('trident', ['verify', 'corrupt.zheng', '--target', 'nox'], 1)
    report['status'] = 'passed'
finally:
    (work/'witness.json').unlink()
    a.output.write_text(json.dumps(report, indent=2) + '\n')
print(json.dumps(dict(status=report['status'], commands=len(report['commands']))))
