"""Measure and exercise an installed soft3-only Joy without other executables."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import tempfile

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--joy', type=Path, required=True)
parser.add_argument('--output', type=Path, required=True)
args = parser.parse_args()
repo = Path(__file__).resolve().parents[2]
root = repo.parent
binary = args.joy.resolve()
work = Path(tempfile.mkdtemp(prefix='joy-soft3-installed-'))
report = dict(commands=[], work=str(work), binary=str(binary),
              source_revisions={}, status='running')
args.output.parent.mkdir(parents=True, exist_ok=True)

def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()

def flush():
    args.output.write_text(json.dumps(report, indent=2) + '\n')

def run(arguments, expected=0, cwd=work):
    command = list(map(str, arguments))
    output = subprocess.run(command, cwd=cwd, capture_output=True, text=True,
                            env=dict(os.environ, PATH='/usr/bin:/bin'), timeout=120)
    report['commands'].append(dict(command=command, cwd=str(cwd),
        exit_code=output.returncode, stdout=output.stdout, stderr=output.stderr))
    flush()
    assert output.returncode == expected, report['commands'][-1]
    return output

for name in ['joy', 'trident', 'nox', 'zheng', 'bbg', 'hemera', 'lens', 'strata', 'honeycrisp', 'neuron']:
    report['source_revisions'][name] = run(['/usr/bin/git', '-C', root/name, 'rev-parse', 'HEAD']).stdout.strip()
report['platform'] = run(['/usr/bin/uname', '-sm']).stdout.strip()
report['macos'] = run(['/usr/bin/sw_vers']).stdout
report['no_foreign_sibling'] = not (root/'trisha').exists()
assert report['no_foreign_sibling']
shutil.copy2(binary, work/'joy')
report['binary_sha256'] = sha(binary)
report['binary_bytes'] = binary.stat().st_size
joy = work/'joy'
package = json.loads(run([joy, 'describe']).stdout)
assert package['runtime']['proof_formats'] == [
    'zheng-nox-public-execution-v2', 'joy-nox-public-state-execution-v1']
(work/'public.tri').write_text('program arithmetic\nfn main(x:Field,y:Field)->Field {(x+y)*x}\n')
run([joy, 'build', 'public.tri', '--profile', 'release', '-o', 'public.json'])
assert run([joy, 'run', 'public.json', '--input-values', '3,5']).stdout == '24\n'
run([joy, 'prove', 'public.json', '--input-values', '3,5', '--output', 'public.zheng'])
run([joy, 'verify', 'public.zheng', '--input-values', '3,5', '--claim', '24'])
run([joy, 'verify', 'public.zheng', '--claim', '25'], 1)
run([joy, 'verify', 'public.zheng', '--budget', '1'], 1)
(work/'secret.tri').write_text('program witness\nfn main()->Field {divine()+1}\n')
assert run([joy, 'run', 'secret.tri', '--secret', '41']).stdout == '42\n'
(work/'keep.zheng').write_bytes(b'preserved')
for request in [['--zk'], ['--secret', '424242']]:
    rejected = run([joy, 'prove', 'missing.tri', '--output', 'keep.zheng', *request], 1)
    assert 'unavailable in soft3-only Joy' in rejected.stderr
    assert '424242' not in rejected.stderr and rejected.stdout == ''
    assert (work/'keep.zheng').read_bytes() == b'preserved'
(work/'sdk.tri').write_text('program sdk\nuse std.compiler.nox.syntax\nfn main()->U32 {syntax.TABLE_CAP}\n')
assert run([joy, 'run', 'sdk.tri']).stdout == '4096\n'
for module in ['std.compiler.codegen', 'std.kernel', 'os.neptune.kernel']:
    (work/'foreign.tri').write_text(f'program foreign\nuse {module}\nfn main()->Field {{1}}\n')
    run([joy, 'build', 'foreign.tri', '-o', 'foreign.json'], 1)
    assert not (work/'foreign.json').exists()
for target in ['triton', 'neptune']:
    run([joy, 'describe', '--target', target], 1)
message = b'Hello, world!\n'
(work/'hello.tri').write_text('program hello\nfn main()->[Field;14] {['+','.join(map(str,message))+']}\n')
run([joy, 'build', 'hello.tri', '--emit', 'nox', '--profile', 'release', '-o', 'hello.nox'])
assert list(map(int,run([joy,'run','hello.nox']).stdout.split())) == list(message)+[0]
shutil.copy2(joy, work/'joy.stripped')
run(['/usr/bin/strip', '-x', 'joy.stripped'])
run(['/usr/bin/codesign', '--force', '--sign', '-', 'joy.stripped'])
assert run([work/'joy.stripped','run','hello.nox']).stdout == run([joy,'run','hello.nox']).stdout
report['stripped_bytes'] = (work/'joy.stripped').stat().st_size
report['stripped_sha256'] = sha(work/'joy.stripped')
report['hello_formula_bytes'] = (work/'hello.nox').stat().st_size
report['hello_formula_sha256'] = sha(work/'hello.nox')
# Retained symbol names give a second check independent of Cargo manifests.
command = ['/usr/bin/xcrun','nm','--demangle','--defined-only',str(binary)]
nm = subprocess.run(command, capture_output=True, text=True, check=True)
forbidden = re.compile(r'(?<![A-Za-z0-9_])(trisha_rs|triton_vm|triton_air|tasm_lib|twenty_first)(::|\.\.)')
report['symbols'] = dict(command=command, output_sha256=hashlib.sha256(nm.stdout.encode()).hexdigest(),
                        forbidden=[line for line in nm.stdout.splitlines() if forbidden.search(line)])
assert not report['symbols']['forbidden']
data = binary.read_bytes()
legacy = sorted((root/'trident/lib/std/compiler').glob('*.tri'))
native = sorted((root/'trident/lib/std/compiler/nox').rglob('*.tri'))
assert all(p.read_bytes() not in data for p in legacy)
assert all(p.read_bytes() in data for p in native)
report['embedded_resources'] = dict(legacy_compiler_files_absent=len(legacy), native_compiler_files_present=len(native))
assert sha(binary) == report['binary_sha256']
report['status'] = 'passed'
flush()
print(json.dumps({k:report[k] for k in ['status','binary_bytes','stripped_bytes','hello_formula_bytes','embedded_resources']}))
