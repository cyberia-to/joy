"""Retain actual Rust1.89 validation, with bounded owned process groups."""
import hashlib
import json
import os
from pathlib import Path
import shutil
import signal
import subprocess
import sys
import time

ROOT = Path(__file__).resolve().parents[2]
OUT = Path(__file__).resolve().parent
BIN = Path('/Users/master/.rustup/toolchains/1.89.0-aarch64-apple-darwin/bin')
FAMILY = ROOT.parent
TARGET = FAMILY.parent / 'census/joy/target'
ENV = dict(PATH=str(BIN) + ':/Users/master/.cargo/bin:/opt/homebrew/bin:/usr/bin:/bin:/usr/sbin:/sbin',
           RUSTC=str(BIN / 'rustc'), RUSTDOC=str(BIN / 'rustdoc'),
           CARGO_TARGET_DIR=str(TARGET), CARGO_BUILD_JOBS='2', CARGO_INCREMENTAL='0',
           RUSTFLAGS='-D warnings', RUSTDOCFLAGS='-D warnings')
COMMANDS = {
    'focused': [str(BIN / 'cargo'), 'test', '-p', 'cyber-joy', '--test', 'structured_certificates',
                '--test', 'structured_run', '--release', '--locked', '--offline', '--', '--test-threads=2'],
    'check': [str(BIN / 'cargo'), 'check', '--workspace', '--all-targets', '--release', '--locked', '--offline'],
    'test': [str(BIN / 'cargo'), 'test', '--workspace', '--release', '--locked', '--offline', '--', '--test-threads=2'],
    'boundary': [sys.executable, '-B', 'scripts/check-soft3-boundary.py'],
    'install': [str(BIN / 'cargo'), 'install', '--path', 'cli', '--root', str(FAMILY / 'installed'),
                '--locked', '--offline', '--force'],
}
COMMANDS['install-path'] = COMMANDS['install']

def identity(p):
    b = p.read_bytes()
    return dict(bytes=len(b), sha256=hashlib.sha256(b).hexdigest())

def git(root, *args):
    return subprocess.check_output(['git', '-C', str(root), *args], text=True).strip()

def sources():
    names = git(ROOT, 'ls-files', '--cached', '--others', '--exclude-standard').splitlines()
    return dict(base=git(ROOT, 'rev-parse', 'HEAD'), files={n: identity(ROOT / n) for n in names
                if not n.startswith('audit/host-proof-deadline/')},
                siblings={p.name: dict(path=str(p.resolve()), revision=git(p, 'rev-parse', 'HEAD'),
                                       status=git(p, 'status', '--porcelain'))
                          for p in sorted(FAMILY.iterdir()) if p.is_symlink()})

def group(pid):
    rows = subprocess.check_output(['/bin/ps', '-axo', 'pid=,pgid=,rss='], text=True)
    return [dict(pid=a, rss_bytes=c*1024) for a,b,c in
            (map(int, line.split()) for line in rows.splitlines()) if b == pid]

def run(kind):
    dest = OUT / kind
    dest.mkdir()
    selected_env = dict(ENV)
    if kind == 'install-path':
        selected_env['PATH'] = str(FAMILY / 'installed/bin') + ':' + ENV['PATH']
    env = dict(os.environ, **selected_env)
    before = sources()
    receipt = dict(schema='joy/host-proof-deadline-gate/v1', status='running', kind=kind,
                   argv=COMMANDS[kind], cwd=str(ROOT), environment=selected_env, sources_before=before,
                   driver=identity(Path(__file__)), started_ns=time.time_ns(),
                   versions={n:subprocess.check_output([str(BIN/n), '--version', '--verbose'],env=env,text=True)
                             for n in ['cargo','rustc']},
                   caps=dict(wall_seconds=1800, sampled_group_rss_bytes=6<<30,
                             logs_bytes=32<<20, free_disk_floor_bytes=8<<30),
                   sampled_peak_group_rss_bytes=0)
    def save():
        (dest/'receipt.json').write_text(json.dumps(receipt, indent=2)+'\n')
    save()
    start=time.monotonic()
    with (dest/'stdout').open('xb') as out, (dest/'stderr').open('xb') as err, \
         (dest/'resources.jsonl').open('x') as samples:
        p=subprocess.Popen(COMMANDS[kind],cwd=ROOT,env=env,stdout=out,stderr=err,start_new_session=True)
        receipt['pid']=p.pid
        try:
            while p.poll() is None:
                rows=group(p.pid); rss=sum(x['rss_bytes'] for x in rows)
                elapsed=time.monotonic()-start; free=shutil.disk_usage(ROOT).free
                receipt['sampled_peak_group_rss_bytes']=max(receipt['sampled_peak_group_rss_bytes'],rss)
                samples.write(json.dumps(dict(time_ns=time.time_ns(),elapsed_seconds=elapsed,
                                              processes=rows,rss_bytes=rss,free_bytes=free))+'\n')
                samples.flush()
                if elapsed>1800 or rss>6<<30 or out.tell()+err.tell()>32<<20 or free<8<<30:
                    receipt['resource_stop']='wall/RSS/log/free-disk cap'
                    os.killpg(p.pid,signal.SIGTERM)
                    try: p.wait(timeout=10)
                    except subprocess.TimeoutExpired: os.killpg(p.pid,signal.SIGKILL)
                    break
                save(); time.sleep(1)
            receipt['exit_code']=p.wait(timeout=10)
        finally:
            if group(p.pid):
                receipt['orphan_group']=True
                os.killpg(p.pid,signal.SIGKILL)
    receipt.update(elapsed_seconds=time.monotonic()-start,sources_after=sources(),
                   files={n:identity(dest/n) for n in ['stdout','stderr','resources.jsonl']})
    receipt['warning_lines']=[line for n in ['stdout','stderr'] for line in (dest/n).read_text().splitlines()
                              if line.startswith('warning:')]
    receipt['status']='passed' if (receipt['exit_code']==0 and not receipt['warning_lines']
        and not receipt.get('resource_stop') and not receipt.get('orphan_group')
        and receipt['sources_before']==receipt['sources_after']) else 'failed'
    if kind.startswith('install') and receipt['status']=='passed':
        receipt['binary']=identity(FAMILY/'installed/bin/joy')
    save()
    print(json.dumps({k:receipt[k] for k in ['kind','status','exit_code','elapsed_seconds']}),flush=True)
    if receipt['status']!='passed': raise SystemExit(1)

if __name__=='__main__':
    for kind in sys.argv[1:]: run(kind)
