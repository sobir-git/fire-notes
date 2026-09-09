#!/usr/bin/env python3
"""Run native Notes benchmarks on private Xvfb/D-Bus with temporary data.

Three ordinary runs always execute. --growth adds separate startup and editing
profiles; their memory growth is reported without applying the ordinary ceiling.
No user display, session bus, notes, or allocator tuning is used.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import time

from memory_probe import FIELDS, LOGICAL_WINDOW_SIZES, TYPED_TEXT, memory_budget, ticks
from private_session import PrivateSession

CHECKS = ('typing', 'animated_fire', 'clipboard_paste', 'undo_paste', 'redo_paste',
          'undo_redone_paste', 'tabs_visited', 'resize_geometry', 'scrolling',
          'quiet_idle', 'restored_session')
GROWTH = [('startup', size, 1) for size in (1024, 8192, 65536, 262144, 1048576)] + [
    ('editing', 8192, 10), ('editing', 65536, 1)]


def digest(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def reference(path, directory):
    return {'path': os.path.relpath(path, directory), 'sha256': digest(path)}


def assess(report, fingerprint, size=8192, tabs=3, scope='ordinary'):
    failures = []
    def check(condition, description):
        if not condition:
            failures.append(description)
    check(report.get('binary_sha256') == fingerprint, 'binary hash mismatch')
    check(not report.get('error'), f"native failure: {report.get('error')}")
    check(not report.get('sampling_errors'), 'sampling errors')
    check(report.get('units') == 'KiB', 'invalid accounting units')
    workload = report.get('workload', {})
    check(workload.get('document_bytes') == size and workload.get('tabs') == tabs,
          'document/tab workload differs')
    check(workload.get('startup_only') is (scope == 'startup'), 'workload scope differs')
    check(report.get('app_args') == [], 'default app capabilities required')
    samples = [*report.get('samples', []), *report.get('memory_samples', {}).values()]
    check(bool(samples) and all(all(type(s.get(f)) is int and s[f] >= 0 for f in FIELDS) for s in samples),
          'incomplete raw memory accounting')
    budget = memory_budget(samples, report.get('sampling_errors', []))
    if scope == 'ordinary':
        check(budget['passed'], 'ordinary memory ceiling or swap check failed')
    else:
        check(not budget.get('invalid_sample_indices'), 'invalid growth accounting')
        check(budget.get('max_sampled_swap_bytes') == 0 and budget.get('max_sampled_swap_pss_bytes') == 0,
              'growth used swap')
    if scope != 'startup':
        check(all(report.get('checks', {}).get(name) is True for name in CHECKS), 'native workload assertion missing')
        check(workload.get('logical_window_sizes') == [list(size) for size in LOGICAL_WINDOW_SIZES]
              and workload.get('typed_text') == TYPED_TEXT, 'window/typing workload differs')
    else:
        check({'startup', 'startup_settled'} <= report.get('memory_samples', {}).keys(),
              'startup did not settle')
    check(report.get('launch_environment', {}).get('allocator_environment') == {}, 'allocator override present')
    return failures


def run_one(binary, output, name, scope='ordinary', size=8192, tabs=3):
    report_path = output / (name+'.json')
    metadata_path = output / (name+'.metadata.json')
    metadata = {'binary_sha256': digest(binary), 'scope': scope, 'document_bytes': size, 'tabs': tabs,
                'display_kind': 'virtual_x11', 'hardware_presentation_measured': False}
    server = None
    try:
        with PrivateSession() as session:
            # Configure the private bus before it activates any desktop services.
            for key in list(session.env):
                if key in ('GLIBC_TUNABLES', 'LD_PRELOAD', 'IBUS_ADDRESS', 'MESA_LOADER_DRIVER_OVERRIDE',
                           'GALLIUM_DRIVER', '__GLX_VENDOR_LIBRARY_NAME') or key.startswith(('MALLOC_', 'MIMALLOC_', 'JEMALLOC_')):
                    session.env.pop(key)
            session.env.update(WINIT_X11_SCALE_FACTOR='2', LC_ALL='C.UTF-8', GSETTINGS_BACKEND='memory', GIO_USE_VFS='local')
            with (session.directory/'display').open('w+') as display_file:
                server = subprocess.Popen(['Xvfb', '-displayfd', str(display_file.fileno()), '-screen', '0',
                                           '2560x1800x24', '-nolisten', 'tcp'], pass_fds=(display_file.fileno(),),
                                          env=session.env, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
                try:
                    number = ''
                    for _ in range(100):
                        display_file.seek(0)
                        number = display_file.read().strip()
                        if number:
                            break
                        if server.poll() is not None:
                            raise RuntimeError('Xvfb exited before creating a display')
                        time.sleep(.05)
                    if not number:
                        raise RuntimeError('Xvfb startup timed out')
                    env = session.activate(':'+number)
                    def accessibility_status(method, *args):
                        command = [
                            'gdbus', 'call', '--session', '--dest', 'org.a11y.Bus',
                            '--object-path', '/org/a11y/bus', '--method',
                            f'org.freedesktop.DBus.Properties.{method}',
                            'org.a11y.Status', 'IsEnabled', *args,
                        ]
                        return subprocess.check_output(
                            command, env=env, text=True, timeout=10,
                        ).strip()

                    accessibility_status('Set', '<true>')
                    metadata['accessibility_enabled_before'] = accessibility_status('Get')
                    if metadata['accessibility_enabled_before'] != '(<true>,)':
                        raise RuntimeError('Private accessibility service is not enabled')
                    server_ticks = ticks(server.pid)
                    started = time.monotonic()
                    args = [sys.executable, str(Path(__file__).with_name('memory_probe.py')), '--binary', str(binary),
                            '--output', str(report_path), '--private-display', '--document-bytes', str(size), '--tabs', str(tabs)]
                    if scope == 'startup':
                        args.append('--startup-only')
                    with (output/(name+'.log')).open('w') as log:
                        process = subprocess.run(args, env=env, stdout=log, stderr=log)
                    metadata.update(probe_exit_code=process.returncode, xvfb_pid=server.pid,
                                    xvfb_cpu_ticks=ticks(server.pid)-server_ticks,
                                    cpu_ticks_per_second=os.sysconf('SC_CLK_TCK'),
                                    xvfb_observation_seconds=time.monotonic()-started,
                                    accessibility_enabled_after=accessibility_status('Get'))
                    if metadata['accessibility_enabled_after'] != '(<true>,)':
                        raise RuntimeError('Private accessibility service became disabled')
                finally:
                    if server.poll() is None:
                        server.terminate()
                    server.wait(timeout=5)
        metadata_path.write_text(json.dumps(metadata, indent=2)+'\n')
        report = json.loads(report_path.read_text())
        report['display_state'] = {'kind': 'private', 'available': True, 'isolated': True, 'provider': 'Xvfb'}
        report['run_context'] = {'accessibility_bus': 'private_enabled', 'software_gl_override': False,
                                 'hardware_presentation_measured': False}
        report['environment_receipt'] = reference(metadata_path, output)
        report_path.write_text(json.dumps(report, indent=2)+'\n')
        # memory_probe still reports overall release acceptance pending; its legacy
        # exit1 alone is not the result of this bounded memory/workload gate.
        failures = assess(report, metadata['binary_sha256'], size, tabs, scope)
        if metadata['probe_exit_code'] not in (0, 1):
            failures.append('probe process failed unexpectedly')
        return reference(report_path, output), failures
    except (OSError, ValueError, KeyError, subprocess.SubprocessError, RuntimeError) as error:
        metadata['error'] = repr(error)
        metadata_path.write_text(json.dumps(metadata, indent=2)+'\n')
        if not report_path.exists():
            report_path.write_text(json.dumps({'binary_sha256': metadata['binary_sha256'], 'error': repr(error)}, indent=2)+'\n')
        return reference(report_path, output), [repr(error)]


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary', default='target/release/fire-notes')
    parser.add_argument('--output', required=True, help='New artifact directory')
    parser.add_argument('--growth', action='store_true', help='Also profile separate document/tab growth workloads')
    args = parser.parse_args(argv)
    binary = Path(args.binary).resolve()
    output = Path(args.output).resolve()
    if output.exists() and any(output.iterdir()):
        parser.error('output directory must be empty; previous evidence is preserved')
    output.mkdir(parents=True, exist_ok=True)
    result = {'schema': 1, 'binary_sha256': digest(binary), 'ordinary': [], 'growth': [],
              'ordinary_passed': True, 'growth_passed': True, 'failures': {},
              'source_context': {'probe_sha256': digest(Path(__file__).with_name('memory_probe.py')),
                                 'harness_sha256': digest(__file__), 'display': 'private Xvfb, no hardware display claim'}}
    for index in range(1, 4):
        name = f'ordinary-{index}'
        ref, failures = run_one(binary, output, name)
        result['ordinary'].append(ref)
        if failures:
            result['ordinary_passed'] = False
            result['failures'][name] = failures
    if args.growth:
        for scope, size, tabs in GROWTH:
            name = f'{scope}-{size}-{tabs}'
            ref, failures = run_one(binary, output, name, scope, size, tabs)
            result['growth'].append({'document_bytes': size, 'tabs': tabs, 'scope': scope, 'report': ref})
            if failures:
                result['growth_passed'] = False
                result['failures'][name] = failures
    (output/'benchmark.json').write_text(json.dumps(result, indent=2)+'\n')
    print(json.dumps(result, indent=2))
    return 0 if result['ordinary_passed'] and result['growth_passed'] else 1


if __name__ == '__main__':
    raise SystemExit(main())
