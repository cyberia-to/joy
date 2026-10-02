"""Run bounded census gates with exact commands, inputs and sampled RSS receipts."""
import hashlib
import json
import os
from pathlib import Path
import signal
import subprocess
import sys
import time

ROOT = Path(__file__).resolve().parents[2]
OUT = Path(__file__).resolve().parent
FAMILY = ROOT.parent
PREP = Path('/Users/master/cyber/.worktrees/selfhost-0.4-full-bootstrap/measurements/selfhost-source-preparation/final/prepared')
OWNED = ['specs/compiler-proof-census.md', 'rs/structured/tests.rs',
         'rs/structured/tests/census.rs', 'rs/structured/tests/census/driver.rs',
         'rs/structured/tests/census/tests.rs']
RSS_KIB = 2 * 1024 * 1024
LOG_LIMIT = 128 << 20


def identity(path):
    raw = path.read_bytes()
    return dict(bytes=len(raw), sha256=hashlib.sha256(raw).hexdigest())


def git(path, *args):
    return subprocess.check_output(['git', '-C', str(path), *args], text=True).strip()


def sources():
    siblings = {}
    for name in ['trident', 'nox', 'zheng', 'bbg', 'hemera', 'neuron', 'lens', 'honeycrisp',
                 'strata/nebu', 'strata/jali', 'strata/kuro', 'strata/genies', 'strata/trop']:
        path = FAMILY / name
        if path.exists():
            siblings[name] = dict(path=str(path.resolve()), revision=git(path, 'rev-parse', 'HEAD'),
                                  status=git(path, 'status', '--porcelain'))
    return dict(revision=git(ROOT, 'rev-parse', 'HEAD'),
                files={name: identity(ROOT / name) for name in OWNED}, siblings=siblings)


def rss_tree(pid):
    rows = [tuple(map(int, row.split())) for row in
            subprocess.check_output(['/bin/ps', '-axo', 'pid=,ppid=,rss='], text=True).splitlines()]
    selected = {pid}
    while True:
        following = selected | {child for child, parent, _ in rows if parent in selected}
        if following == selected:
            break
        selected = following
    return sum(rss for child, _, rss in rows if child in selected)


kind = sys.argv[1]
if len(sys.argv) > 2 and not sys.argv[2].isdigit():
    raise ValueError('attempt suffix must be decimal')
label = kind + ('-' + sys.argv[2] if len(sys.argv) > 2 else '')
commands = {
    'format': ['rustfmt', '--check', '--edition', '2021', '--config', 'skip_children=true', *OWNED[1:]],
    'boundary': ['python3', 'scripts/check-soft3-boundary.py'],
    'check': ['cargo', 'check', '--workspace', '--all-targets', '--release', '--locked', '--offline'],
    'focused': ['cargo', 'test', '-p', 'joy-rs', '--lib', '--release', '--locked', '--offline',
                'structured::tests::census', '--', '--nocapture', '--test-threads=2'],
    'workspace': ['cargo', 'test', '--workspace', '--release', '--locked', '--offline', '--', '--test-threads=2'],
}
env = dict(os.environ, CARGO_BUILD_JOBS='2', CARGO_TARGET_DIR=str(FAMILY / 'target'))
input_files = {}
if kind in ('prefix100k', 'prefix1m'):
    for name, expected in [('compiler.dag', '76a07c08265bd2ef525164472b6b53ac3f0e6cbbedce3250c4202f40ffba34c8'),
                           ('job.dag', '3474e5583e7b3ac36bdd526435bb2ae584691774a009e29ca02407c22589f13d')]:
        input_files[name] = dict(path=str(PREP / name), **identity(PREP / name))
        assert input_files[name]['sha256'] == expected, 'changed frozen input'
    env.update(JOY_CENSUS_PROGRAM=str(PREP / 'compiler.dag'), JOY_CENSUS_INPUT=str(PREP / 'job.dag'),
               JOY_CENSUS_OUTPUT=str(OUT / f'{label}.json'),
               JOY_CENSUS_TRANSITIONS='100000' if kind == 'prefix100k' else '1000000')
    assert not Path(env['JOY_CENSUS_OUTPUT']).exists()
    argv = ['cargo', 'test', '-p', 'joy-rs', '--lib', '--release', '--locked', '--offline',
            'structured::tests::census::driver::frozen_compiler_prefix', '--', '--ignored', '--exact', '--nocapture']
else:
    argv = commands[kind]

receipt_path = OUT / f'{label}-command.json'
assert not receipt_path.exists(), 'retain existing receipts; use a new label for another attempt'
report = dict(schema=1, scope='bounded compiler census; no proof acceptance', label=label,
              argv=argv, cwd=str(ROOT), environment={key: env[key] for key in env if key.startswith(('JOY_CENSUS_', 'CARGO_'))},
              started_ns=time.time_ns(), sources_before=sources(), inputs=input_files,
              rss_limit_kib=RSS_KIB, rss_sampling_ms=250, log_limit_bytes=LOG_LIMIT, sampled_peak_tree_rss_kib=0,
              runner=identity(Path(__file__)), status='running')


def flush():
    receipt_path.write_text(json.dumps(report, indent=2) + '\n')


flush()
with (OUT / f'{label}.stdout').open('xb') as stdout, (OUT / f'{label}.stderr').open('xb') as stderr:
    process = subprocess.Popen(argv, cwd=ROOT, env=env, stdout=stdout, stderr=stderr, start_new_session=True)
    while process.poll() is None:
        rss = rss_tree(process.pid)
        report['sampled_peak_tree_rss_kib'] = max(report['sampled_peak_tree_rss_kib'], rss)
        if rss > RSS_KIB or stdout.tell() + stderr.tell() > LOG_LIMIT:
            report['guard_failure'] = 'RSS' if rss > RSS_KIB else 'logs'
            os.killpg(process.pid, signal.SIGKILL)
        flush()
        time.sleep(0.25)
    report['exit_code'] = process.wait()
report.update(status='passed' if report['exit_code'] == 0 and 'guard_failure' not in report else 'failed',
              ended_ns=time.time_ns(), sources_after=sources(),
              stdout=identity(OUT / f'{label}.stdout'), stderr=identity(OUT / f'{label}.stderr'))
if report['sources_before'] != report['sources_after']:
    report['status'] = 'source-changed'
for name, old in input_files.items():
    assert identity(Path(old['path'])) == {k: old[k] for k in ['bytes', 'sha256']}, name
raw = (OUT / f'{label}.stdout').read_text() + (OUT / f'{label}.stderr').read_text()
report['warning_lines'] = [line for line in raw.splitlines() if line.startswith('warning:')]
flush()
print(json.dumps({key: report[key] for key in ['label', 'status', 'exit_code', 'sampled_peak_tree_rss_kib', 'warning_lines']}))
raise SystemExit(0 if report['status'] == 'passed' and not report['warning_lines'] else 1)
