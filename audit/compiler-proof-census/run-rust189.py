"""Baseline validation with explicit toolchain, isolated target and bounded RSS."""
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
BIN = Path('/Users/master/.rustup/toolchains/1.89.0-aarch64-apple-darwin/bin')
RUSTUP = '/Users/master/.cargo/bin/rustup'
PREVIOUS = json.loads((OUT / 'prefix1m-2-command.json').read_text())


def identity(path):
    raw = path.read_bytes()
    return dict(bytes=len(raw), sha256=hashlib.sha256(raw).hexdigest())


def git(path, *args):
    return subprocess.check_output(['git', '-C', str(path), *args], text=True).strip()


def sources():
    return dict(revision=git(ROOT, 'rev-parse', 'HEAD'),
                files={name: identity(ROOT / name) for name in PREVIOUS['sources_after']['files']},
                siblings={name: dict(path=old['path'], revision=git(old['path'], 'rev-parse', 'HEAD'),
                                     status=git(old['path'], 'status', '--porcelain'))
                          for name, old in PREVIOUS['sources_after']['siblings'].items()})


def rss_tree(pid):
    rows = [tuple(map(int, row.split())) for row in
            subprocess.check_output(['/bin/ps', '-axo', 'pid=,ppid=,rss='], text=True).splitlines()]
    selected = {pid}
    while True:
        following = selected | {child for child, parent, _ in rows if parent in selected}
        if following == selected:
            return sum(rss for child, _, rss in rows if child in selected)
        selected = following


kind = sys.argv[1]
commands = {
    'check': ['check', '--workspace', '--all-targets', '--release', '--locked', '--offline'],
    'focused': ['test', '-p', 'joy-rs', '--lib', '--release', '--locked', '--offline',
                'structured::tests::census', '--', '--nocapture', '--test-threads=2'],
    'prefix1m': PREVIOUS['argv'][1:],
}
label = 'rust189-' + kind
argv = [RUSTUP, 'run', '1.89.0', str(BIN / 'cargo'), *commands[kind]]
env = dict(os.environ, PATH=str(BIN) + ':' + os.environ['PATH'],
           RUSTUP_TOOLCHAIN='1.89.0', RUSTC=str(BIN / 'rustc'), RUSTDOC=str(BIN / 'rustdoc'),
           CARGO_BUILD_JOBS='2', CARGO_TARGET_DIR=str(FAMILY / 'target-rust189'))
inputs = PREVIOUS['inputs'] if kind == 'prefix1m' else {}
for old in inputs.values():
    assert identity(Path(old['path'])) == {key: old[key] for key in ['bytes', 'sha256']}
if kind == 'prefix1m':
    env.update({key: value for key, value in PREVIOUS['environment'].items() if key.startswith('JOY_CENSUS_')})
    env['JOY_CENSUS_OUTPUT'] = str(OUT / (label + '.json'))
    assert not Path(env['JOY_CENSUS_OUTPUT']).exists()
path = OUT / (label + '-command.json')
assert not path.exists()
report = dict(schema=1, scope='Rust 1.89 compatibility; host census, no proof acceptance',
              label=label, argv=argv, cwd=str(ROOT), started_ns=time.time_ns(),
              environment={key: value for key, value in env.items()
                           if key == 'PATH' or key.startswith(('RUST', 'CARGO', 'JOY_CENSUS_'))},
              toolchain={name: dict(path=str(BIN / name), **identity(BIN / name),
                                    version=subprocess.check_output([str(BIN / name), '--version', '--verbose'],
                                                                    env=env, text=True))
                         for name in ['cargo', 'rustc']}, sources_before=sources(), inputs=inputs,
              runner=dict(path=str(Path(__file__)), **identity(Path(__file__))),
              rss_limit_kib=2 * 1024 * 1024, rss_sampling_ms=250, log_limit_bytes=128 << 20,
              sampled_peak_tree_rss_kib=0, status='running')
with (OUT / (label + '.stdout')).open('xb') as stdout, (OUT / (label + '.stderr')).open('xb') as stderr:
    process = subprocess.Popen(argv, cwd=ROOT, env=env, stdout=stdout, stderr=stderr, start_new_session=True)
    while process.poll() is None:
        report['sampled_peak_tree_rss_kib'] = max(report['sampled_peak_tree_rss_kib'], rss_tree(process.pid))
        if report['sampled_peak_tree_rss_kib'] > report['rss_limit_kib'] or stdout.tell() + stderr.tell() > report['log_limit_bytes']:
            report['guard_failure'] = 'RSS or logs'
            os.killpg(process.pid, signal.SIGKILL)
        path.write_text(json.dumps(report, indent=2) + '\n')
        time.sleep(0.25)
    report['exit_code'] = process.wait()
report.update(ended_ns=time.time_ns(), sources_after=sources(),
              stdout=identity(OUT / (label + '.stdout')), stderr=identity(OUT / (label + '.stderr')))
report['warning_lines'] = [line for suffix in ['stdout', 'stderr']
                           for line in (OUT / (label + '.' + suffix)).read_text().splitlines()
                           if line.startswith('warning:')]
for old in inputs.values():
    assert identity(Path(old['path'])) == {key: old[key] for key in ['bytes', 'sha256']}
report['status'] = ('passed' if report['exit_code'] == 0 and not report['warning_lines']
                    and 'guard_failure' not in report and report['sources_before'] == report['sources_after'] else 'failed')
path.write_text(json.dumps(report, indent=2) + '\n')
print(json.dumps({key: report[key] for key in ['label', 'status', 'exit_code', 'sampled_peak_tree_rss_kib', 'warning_lines']}))
raise SystemExit(0 if report['status'] == 'passed' else 1)
