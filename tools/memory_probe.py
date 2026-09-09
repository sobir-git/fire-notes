#!/usr/bin/env python3
"""Fresh native editor memory acceptance. All notes and session data are temporary."""
import argparse
import errno
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import threading
import time
from native_input import NativeInput

from memory_policy import LIMIT_BYTES as PRIVATE_MEMORY_LIMIT_BYTES, TARGET_BYTES
FIELDS = ('Rss', 'Pss', 'Private_Clean', 'Private_Dirty', 'Shared_Clean', 'Shared_Dirty', 'Anonymous', 'Swap', 'SwapPss')
LOGICAL_WINDOW_SIZES = ((600, 400), (900, 600), (1200, 800))
TYPED_TEXT = 'typing memory workload. ' * 13


def infer_scale(physical, logical):
    horizontal = physical[0] / logical[0]
    vertical = physical[1] / logical[1]
    if min(horizontal, vertical) <= 0 or abs(horizontal-vertical) > .01:
        raise ValueError(f'Cannot infer uniform scale from physical {physical}, logical {logical}')
    return horizontal


def physical_size(logical, scale):
    return tuple(round(value * scale) for value in logical)


def wait_until(predicate, message, timeout=5):
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        if predicate(): return
        time.sleep(.02)
    raise AssertionError(message)


def document_evidence(actual, expected):
    return {'equal': actual == expected, 'actual_bytes': len(actual), 'expected_bytes': len(expected),
            'actual_sha256': hashlib.sha256(actual).hexdigest(),
            'expected_sha256': hashlib.sha256(expected).hexdigest()}


def confirmed_clean_exit_race(error, process, expected_exit):
    """Only a disappearing /proc entry followed by an expected exit(0) is benign."""
    if not expected_exit or not isinstance(error, OSError) or error.errno not in (errno.ESRCH, errno.ENOENT):
        return False
    try:
        # Linux can drop the mm before waitpid reports completion. Do not infer
        # successful exit from ESRCH alone or hide errors from a still-live app.
        return process.wait(timeout=.1) == 0
    except subprocess.TimeoutExpired:
        return False


def memory(pid):
    fields = {}
    for line in Path(f'/proc/{pid}/smaps_rollup').read_text().splitlines()[1:]:
        name, value = line.split(':', 1)
        fields[name] = int(value.split()[0])
    return {name: fields[name] for name in FIELDS}


def memory_budget(samples, sampling_errors=()):
    values = samples.values() if isinstance(samples, dict) else samples
    values = list(values)
    required = ('Private_Dirty', 'Private_Clean', 'Swap', 'SwapPss')
    invalid = [i for i, sample in enumerate(values)
               if any(type(sample.get(key)) is not int or sample[key] < 0 for key in required)]
    if invalid:
        return {'metric': 'Private_Dirty', 'limit_bytes_exclusive': PRIVATE_MEMORY_LIMIT_BYTES,
                'passed': False, 'invalid_sample_indices': invalid}
    peak = max((s['Private_Dirty'] for s in values), default=0) * 1024
    swap = max((s['Swap'] for s in values), default=0) * 1024
    swap_pss = max((s['SwapPss'] for s in values), default=0) * 1024
    return {'metric': 'Private_Dirty', 'limit_bytes_exclusive': PRIVATE_MEMORY_LIMIT_BYTES,
            'target_bytes_exclusive': TARGET_BYTES,
            'target_met': bool(values) and not sampling_errors and peak < TARGET_BYTES and swap == 0 and swap_pss == 0,
            'max_sampled_bytes': peak, 'max_sampled_swap_bytes': swap,
            'max_sampled_swap_pss_bytes': swap_pss,
            'sampling_error_count': len(sampling_errors),
            'max_total_private_resident_bytes': max((s['Private_Dirty']+s['Private_Clean'] for s in values), default=0)*1024,
            'passed': bool(values) and not sampling_errors and peak < PRIVATE_MEMORY_LIMIT_BYTES and swap == 0 and swap_pss == 0}


def ticks(pid):
    fields = Path(f'/proc/{pid}/stat').read_text().rsplit(')', 1)[1].split()
    return int(fields[11]) + int(fields[12])


def launch_environment(env):
    """Record cost-affecting overrides without dumping unrelated environment data."""
    allocator = {key: value for key, value in sorted(env.items()) if value and
                 (key in ('GLIBC_TUNABLES', 'LD_PRELOAD') or
                  key.startswith(('MALLOC_', 'MIMALLOC_', 'JEMALLOC_')))}
    return {'allocator_environment': allocator,
            'allocator_environment_is_default': not allocator,
            'graphics_environment': {key: env[key] for key in
                ('LIBGL_ALWAYS_SOFTWARE', 'MESA_LOADER_DRIVER_OVERRIDE', 'GALLIUM_DRIVER',
                 '__GLX_VENDOR_LIBRARY_NAME', 'WINIT_X11_SCALE_FACTOR') if key in env}}


def cpu_summary(samples, ticks_per_second):
    """Keep process lifetimes separate and do not attribute transition intervals."""
    groups = {}
    for item in samples:
        groups.setdefault((item['launch'], item['pid']), []).append(item)
    total_ticks = 0; observed_seconds = 0.; stages = {}; launches = []
    for (launch, pid), values in groups.items():
        if len(values) < 2: continue
        duration = values[-1]['t']-values[0]['t']
        used_ticks = values[-1]['cpu_ticks']-values[0]['cpu_ticks']
        if duration <= 0 or used_ticks < 0:
            raise ValueError('Invalid process CPU sample sequence')
        total_ticks += used_ticks; observed_seconds += duration
        launches.append({'launch': launch, 'pid': pid, 'cpu_seconds': used_ticks/ticks_per_second,
                         'observed_seconds': duration})
        for before, after in zip(values, values[1:]):
            elapsed = after['t']-before['t']; delta = after['cpu_ticks']-before['cpu_ticks']
            if elapsed < 0 or delta < 0:
                raise ValueError('Non-monotonic process CPU samples')
            if before['stage'] != after['stage']: continue
            stage = stages.setdefault(before['stage'], {'cpu_ticks': 0, 'observed_seconds': 0., 'intervals': 0})
            stage['cpu_ticks'] += delta; stage['observed_seconds'] += elapsed; stage['intervals'] += 1
    for value in stages.values():
        value['cpu_seconds'] = value['cpu_ticks']/ticks_per_second
        value['one_core_percent'] = (100*value['cpu_seconds']/value['observed_seconds']
                                     if value['observed_seconds'] else None)
    return {'available': bool(launches), 'ticks_per_second': ticks_per_second,
            'sampled_process_cpu_seconds': total_ticks/ticks_per_second,
            'observed_process_seconds': observed_seconds, 'launches': launches, 'stages': stages,
            'limits': ['Process CPU includes app rendering, driver and accessibility threads.',
                      'Launch prefixes and exit tails outside the samples are omitted.',
                      'Stage totals omit intervals crossing a stage boundary; total process CPU includes them.',
                      'Stages include probe waits and can group several actions; these are not input latency measurements.']}


def display_processes():
    result = []
    for proc in Path('/proc').glob('[0-9]*'):
        try:
            name = (proc/'comm').read_text().strip()
            if name not in ('Xorg', 'Xwayland', 'Xvfb', 'gnome-shell', 'kwin_x11', 'picom', 'xfwm4', 'mutter-x11-fram'):
                continue
            result.append(int(proc.name))
        except OSError: pass
    return result


def external_costs(env, phase, app_pid=None):
    """Isolate Xlib errors and preserve unavailable evidence instead of reporting zero."""
    pids = set(display_processes())
    if app_pid is not None: pids.add(app_pid)
    record = {'phase': phase, 'timestamp_ns': time.time_ns(), 'app_pid': app_pid,
              'requested_pids': sorted(pids)}
    if not pids:
        return {**record, 'available': False, 'error': 'No display processes found'}
    command = [sys.executable, str(Path(__file__).with_name('memory_accounting.py')),
               '--display', env.get('DISPLAY', ':0')]
    for pid in sorted(pids): command.extend(('--pid', str(pid)))
    try:
        run = subprocess.run(command, env=env, capture_output=True, text=True, timeout=15)
        if run.returncode:
            return {**record, 'available': False, 'error': run.stderr, 'exit_code': run.returncode}
        data = json.loads(run.stdout)
        process_errors = [p for p in data['processes'] if any(
            key in p for key in ('memory_error', 'drm_error', 'fd_errors'))]
        observed_pids = {p['pid'] for p in data['processes']}
        app_clients = [c for c in data['xres_clients'] if c['pid'] == app_pid]
        return {**record, 'available': not data['query_errors'] and not process_errors
                and observed_pids == pids and (app_pid is None or bool(app_clients)), 'accounting': data,
                'missing_process_pids': sorted(pids-observed_pids),
                'app_xres_client_count': len(app_clients)}
    except (OSError, subprocess.TimeoutExpired, ValueError, KeyError) as error:
        return {**record, 'available': False, 'error': repr(error)}


def external_series(env, phase):
    records = []
    for index in range(3):
        if index: time.sleep(.25)
        records.append(external_costs(env, phase))
    return records


def fixture(directory, count, size):
    phrase = 'Fire Notes café e\u0301 Привет العربية שלום. A useful note with ordinary words.\n'
    text = (phrase * (size//len(phrase.encode())+1)).encode()[:size].decode('utf-8', errors='ignore')
    text += ' ' * (size-len(text.encode()))
    paths = []
    for i in range(count):
        path = directory/f'note-{i+1}.md'
        path.write_text(text if i != 1 else text.replace('\n', ' '))
        paths.append(str(path))
    state = dict(tabs=paths, active=paths[0], titles={}, position=None, width=600., height=400.,
                 views={p: dict(caret=0, anchor=None, x=0., y=0., wrap=i != 1) for i,p in enumerate(paths)})
    (directory/'session.json').write_text(json.dumps(state))
    return text, paths


def desktop_state(env):
    """Read the session lock; never unlock it or submit input to a locked desktop."""
    errors = []
    for name in ('org.gnome.ScreenSaver', 'org.freedesktop.ScreenSaver'):
        path = '/' + name.replace('.', '/')
        try:
            run = subprocess.run(['gdbus', 'call', '--session', '--dest', name,
                '--object-path', path, '--method', name+'.GetActive'], env=env,
                capture_output=True, text=True, timeout=3)
            if run.returncode == 0 and run.stdout.strip() in ('(true,)', '(false,)'):
                return {'available': True, 'locked': run.stdout.strip() == '(true,)', 'provider': name}
            errors.append(run.stderr.strip())
        except (OSError, subprocess.TimeoutExpired) as error:
            errors.append(str(error))
    return {'available': False, 'errors': errors}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary', default='target/release/fire-notes')
    parser.add_argument('--output', required=True)
    parser.add_argument('--document-bytes', type=int, default=8192)
    parser.add_argument('--tabs', type=int, default=3)
    parser.add_argument('--private-display', action='store_true', help='Diagnostic private X server; cannot establish desktop presentation acceptance')
    parser.add_argument('--startup-only', action='store_true', help='Diagnostic growth sample; cannot pass complete acceptance')
    parser.add_argument('app_args', nargs=argparse.REMAINDER)
    args = parser.parse_args()
    if args.tabs < 1 or args.document_bytes < 1:
        parser.error('--tabs and --document-bytes must be positive')
    binary = Path(args.binary).resolve()
    # Complete build output I/O before measuring. Report private clean pages separately.
    with binary.open("rb") as executable: os.fsync(executable.fileno())
    output = Path(args.output).resolve(); output.parent.mkdir(parents=True, exist_ok=True)
    app_args = args.app_args[1:] if args.app_args[:1] == ['--'] else args.app_args
    env = dict(os.environ)
    env.pop('FIRE_UI_INSPECT', None); env.pop('FIRE_UI_PROFILE', None)
    display_state = {'kind': 'private'} if args.private_display else {'kind': 'desktop', **desktop_state(env)}
    if not args.private_display and (not display_state.get('available') or display_state.get('locked')):
        raise RuntimeError('Desktop lock state unavailable or locked; no app launched and no input sent. Unlock the desktop for acceptance, or use --private-display on an isolated X server for diagnostics.')
    previous = subprocess.run(['xdotool', 'getactivewindow'], env=env, capture_output=True, text=True)
    series = []; checkpoints = {}; sampling_errors = []; monitors = []
    expected_exit_pids = set(); sampling_lifecycle_events = []
    launches = []; stage = 'startup'; started = time.monotonic(); stop = threading.Event()
    result = {'binary': str(binary), 'binary_sha256': hashlib.sha256(binary.read_bytes()).hexdigest(),
              'units': 'KiB', 'display': env.get('DISPLAY'), 'display_state': display_state, 'app_args': app_args,
              'launch_environment': launch_environment(env), 'sampler_cpu': [],
              'run_context': {'accessibility_bus': 'private' if args.private_display else 'normal',
                              'software_gl_override': any(env.get(key) for key in
                                  ('LIBGL_ALWAYS_SOFTWARE', 'MESA_LOADER_DRIVER_OVERRIDE', 'GALLIUM_DRIVER'))},
              'workload': {'document_bytes': args.document_bytes, 'tabs': args.tabs, 'startup_only': args.startup_only,
                           'logical_window_sizes': LOGICAL_WINDOW_SIZES, 'typed_text': TYPED_TEXT},
              'sample_interval_seconds': .025, 'external_before': external_series(env, 'before'), 'checks': {},
              'measurement_limits': ['Sampled peaks can miss allocations shorter than 25ms.',
                 'Sampling starts immediately after Popen; fork/exec before its return is not observable here.',
                 'Display process deltas include unrelated desktop activity.',
                 'External records use bytes for process memory and preserve DRM counter units; app samples use KiB.',
                 'External process swap predates this app; compare resident and swap before/during/after together.',
                 'XRes includes window backing pixmaps here, but GLYPHSET/WINDOW/PICTURE direct byte estimates can be zero despite real cost.',
                 'DRM duplicate descriptors are deduplicated by device/client ID. Buffers shared by different clients and process/DRM/XRes figures overlap; do not sum them.',
                 'XRes and external counter queries run outside the app; checkpoint collection adds time to the workload.']}
    app = None
    def x(*values):
        return subprocess.check_output(['xdotool', *map(str, values)], env=env, stderr=subprocess.DEVNULL, text=True).strip()
    def checkpoint(name):
        nonlocal stage
        stage = name; checkpoints[name] = memory(app.pid)
        result.setdefault('external_checkpoints', {})[name] = external_costs(env, name, app.pid)
        result.setdefault('session_checkpoints', {})[name] = json.loads((notes/'session.json').read_text())
        result.setdefault('pointer_checkpoints', {})[name] = dict(
            line.split('=', 1) for line in x('getmouselocation', '--shell').splitlines())
    def sample(process, launch_id):
        cpu_started = time.thread_time()
        observer = {'launch': launch_id, 'cpu_seconds': None, 'attempts': 0}
        result['sampler_cpu'].append(observer)
        while not stop.is_set() and process.poll() is None:
            observer['attempts'] += 1
            try:
                series.append(dict(t=time.monotonic()-started, stage=stage,
                                   pid=process.pid, launch=launch_id, cpu_ticks=ticks(process.pid), **memory(process.pid)))
            except (OSError, ValueError, KeyError) as error:
                record = dict(t=time.monotonic()-started, pid=process.pid,
                              launch=launch_id, error=repr(error))
                if confirmed_clean_exit_race(error, process, process.pid in expected_exit_pids):
                    sampling_lifecycle_events.append({**record, 'kind': 'confirmed_expected_clean_exit', 'exit_code': 0})
                else:
                    sampling_errors.append(dict(t=time.monotonic()-started, pid=process.pid,
                                                launch=launch_id, error=repr(error)))
            if stop.wait(.025): break
        observer['cpu_seconds'] = time.thread_time()-cpu_started
    with tempfile.TemporaryDirectory(prefix='fire-notes-memory-') as temporary:
        directory = Path(temporary); notes = directory/'notes'; notes.mkdir()
        original, paths = fixture(notes, args.tabs, args.document_bytes)
        env.update(XDG_DATA_HOME=str(directory/'data'), XDG_CONFIG_HOME=str(directory/'config'),
                   XDG_CACHE_HOME=str(directory/'cache'))
        log = (directory/'native.log').open('w+')
        def launch():
            nonlocal app
            app = subprocess.Popen([str(binary), '--data-dir', str(notes), *app_args], env=env, stdout=log, stderr=log)
            launch_id = len(launches)
            launches.append({'pid': app.pid, 'launch': launch_id, 't': time.monotonic()-started})
            monitor = threading.Thread(target=sample, args=(app, launch_id))
            monitors.append(monitor)
            monitor.start()
            for _ in range(100):
                assert app.poll() is None, 'App exited before its window appeared'
                found = subprocess.run(['xdotool', 'search', '--all', '--pid', str(app.pid), '--name', '^Fire Notes$'], env=env, capture_output=True, text=True)
                if found.returncode == 0: return found.stdout.splitlines()[0]
                time.sleep(.05)
            raise AssertionError('Window did not appear')
        try:
            window = launch()
            def focus():
                if not args.private_display:
                    state = desktop_state(env)
                    assert state.get("available") and not state.get("locked"), "Desktop locked or lock state unavailable; refusing native input"
                x('windowactivate' if previous.returncode == 0 else 'windowfocus', '--sync', window)
                for _ in range(30):
                    if x('getwindowfocus') == window: return
                    time.sleep(.02)
                raise AssertionError('Test window lost focus; refusing keyboard input')
            def key(*keys):
                focus()
                with NativeInput(env['DISPLAY']) as native:
                    for command in keys:
                        evidence = {'stage': stage, 'pid': app.pid}
                        result.setdefault('key_injections', []).append(evidence)
                        native.chord(window, command, evidence)
            def quit_app():
                expected_exit_pids.add(app.pid)
                key('ctrl+q')
                app.wait(timeout=10)
                assert app.returncode == 0, 'App did not quit cleanly'
            def geometry():
                values = dict(line.split('=', 1) for line in x('getwindowgeometry', '--shell', window).splitlines())
                return tuple(int(values[key]) for key in ('X', 'Y', 'WIDTH', 'HEIGHT'))
            def session():
                return json.loads((notes/'session.json').read_text())
            def verify_document(name, expected):
                expected_bytes = expected.encode('utf-8')
                key('ctrl+s')
                try:
                    wait_until(lambda: Path(paths[0]).read_bytes() == expected_bytes,
                               f'{name}: saved document differs from exact expected UTF-8 bytes')
                finally:
                    result.setdefault('document_checks', {})[name] = document_evidence(
                        Path(paths[0]).read_bytes(), expected_bytes)
                    if Path(paths[0]).read_bytes() != expected_bytes:
                        output.with_name(output.stem+'-'+name+'-actual.txt').write_bytes(Path(paths[0]).read_bytes())
                        output.with_name(output.stem+'-'+name+'-expected.txt').write_bytes(expected_bytes)
                result['checks'][name] = True
            def expect_active(path):
                wait_until(lambda: session()['active'] == path,
                           f'Tab visit did not persist expected active path {path}')
                state = session()
                assert state['tabs'] == paths, 'Tab order changed while cycling'
                result.setdefault('tab_visits', []).append({'active': state['active'], 'tabs': state['tabs']})
            def shot(name):
                from PIL import ImageGrab
                left, top, width, height = geometry()
                image = ImageGrab.grab(bbox=(left, top, left+width, top+height), xdisplay=env['DISPLAY'])
                image.save(output.with_name(output.stem+'-'+name+'.png'))
                return image
            time.sleep(1); checkpoint('startup')
            initial_physical = geometry()[2:]
            scale = infer_scale(initial_physical, LOGICAL_WINDOW_SIZES[0])
            result['workload']['startup_physical_size'] = initial_physical
            result['workload']['observed_scale'] = scale
            result['workload']['physical_window_sizes'] = [physical_size(size, scale) for size in LOGICAL_WINDOW_SIZES]
            if not args.startup_only:
                key('ctrl+Home')
                focus(); stage = 'typing_fire'
                result['typing_injection'] = {}
                with NativeInput(env['DISPLAY']) as native:
                    native.send(window, text=TYPED_TEXT, delay=.01, evidence=result['typing_injection'])
                checkpoint('typed_fire'); shot('typed-fire'); time.sleep(2)
                typed_document = TYPED_TEXT + original
                verify_document('typing', typed_document)
                key('ctrl+a'); stage='selected_fire'; time.sleep(.2)
                first=shot('selected-fire'); time.sleep(.3); second=shot('selected-fire-next')
                from PIL import ImageChops
                assert ImageChops.difference(first, second).getbbox(), 'Selected fire did not animate'
                result['checks']['animated_fire'] = True
                time.sleep(9.5); checkpoint('selected_fire')
                key('ctrl+c'); key('ctrl+End'); key('ctrl+v')
                verify_document('clipboard_paste', typed_document + typed_document)
                checkpoint('clipboard_paste')
                key('ctrl+z'); verify_document('undo_paste', typed_document)
                key('ctrl+y'); verify_document('redo_paste', typed_document + typed_document)
                key('ctrl+z'); verify_document('undo_redone_paste', typed_document)
                for index in range(args.tabs):
                    key('ctrl+Tab')
                    expect_active(paths[(index+1) % args.tabs])
                result['checks']['tabs_visited'] = True
                checkpoint('tabs')
                key('ctrl+Home')
                focus()
                x('mousemove', '--sync', '--window', window, round(350*scale), round(250*scale))
                checkpoint('before_scroll')
                result['wheel_injection'] = {}
                with NativeInput(env['DISPLAY']) as native:
                    native.send(window, button=5, count=15, delay=.02, evidence=result['wheel_injection'])
                time.sleep(.3); checkpoint('scrolled'); shot('scrolled')
                # Xdotool dimensions are physical pixels. Keep the frozen logical workload at HiDPI.
                for logical in (*LOGICAL_WINDOW_SIZES[1:], LOGICAL_WINDOW_SIZES[0]):
                    width, height = physical_size(logical, scale)
                    stage=f'resize_logical_{logical[0]}x{logical[1]}'
                    at=time.monotonic();x('windowsize', '--sync', window, width, height)
                    command_finished=time.monotonic()
                    wait_until(lambda: geometry()[2:] == (width, height), 'Native window did not reach requested physical size')
                    geometry_observed=time.monotonic()
                    wait_until(lambda: abs(session()['width']-logical[0]) < 1 and abs(session()['height']-logical[1]) < 1,
                               'App did not persist requested logical window size')
                    app_observed=time.monotonic()
                    result.setdefault('resize_observations', []).append({
                        'logical_size': logical, 'physical_size': (width, height),
                        'xdotool_command_seconds': command_finished-at,
                        'request_to_observed_x_geometry_seconds': geometry_observed-at,
                        'request_to_persisted_app_geometry_seconds': app_observed-at,
                        'includes': 'Command launch, X11 synchronization and observer polling; app geometry also includes persistence I/O.',
                        'render_or_presentation_latency': False})
                    time.sleep(.2); checkpoint(stage); shot(stage)
                result['checks']['resize_geometry'] = True
                result['resize_render_latency'] = {'available': False,
                    'reason': 'This memory run does not enable host profiling or measure frame presentation. Command/geometry observations are not rendering latency.'}
                # Resizing persists view state, allowing scrolling to be checked without an inspector.
                scrolled = session()['views'][paths[0]]
                assert scrolled['y'] > 0, 'Native wheel scrolling did not persist a positive scroll offset'
                result['checks']['scrolling'] = True
                result['scrolled_view'] = scrolled
                time.sleep(2)
                idle_started=time.monotonic(); before=ticks(app.pid); time.sleep(2)
                idle_ticks=ticks(app.pid)-before; idle_elapsed=time.monotonic()-idle_started
                cpu_seconds=idle_ticks/os.sysconf('SC_CLK_TCK')
                result['idle']={'elapsed_seconds': idle_elapsed, 'cpu_ticks': idle_ticks,
                    'ticks_per_second': os.sysconf('SC_CLK_TCK'), 'cpu_seconds': cpu_seconds,
                    'one_core_percent': 100*cpu_seconds/idle_elapsed, 'limit_cpu_ticks': 2,
                    'passed': idle_ticks <= 2}
                assert result['idle']['passed'], f'Idle used {idle_ticks} CPU ticks over {idle_elapsed:.3f}s'
                result['checks']['quiet_idle'] = True
                quit_app()
                saved=json.loads((notes/'session.json').read_text());assert len(saved['tabs'])==args.tabs
                stage='restoring';window=launch();time.sleep(1);checkpoint('restored');shot('restored')
                assert geometry()[2:] == initial_physical, 'Restored window has different physical dimensions'
                quit_app()
                restored=json.loads((notes/'session.json').read_text());assert restored['tabs']==saved['tabs'] and restored['active']==saved['active']
                for field in ('views', 'width', 'height', 'titles'):
                    assert restored[field] == saved[field], f'Restored session changed {field}'
                result['checks']['restored_session']=True
                result['session_before_restart']=saved
                result['session_after_restart']=restored
            else:
                time.sleep(1);checkpoint('startup_settled')
        except Exception as error:
            result['error'] = repr(error)
        finally:
            stop.set()
            for monitor in monitors: monitor.join()
            # Preserve temporary evidence even when a native assertion stops the workload.
            try:
                result['last_temporary_session'] = json.loads((notes/'session.json').read_text())
                result['last_temporary_documents'] = [
                    {'path': path, 'bytes': len(Path(path).read_bytes()),
                     'sha256': hashlib.sha256(Path(path).read_bytes()).hexdigest(),
                     'prefix': Path(path).read_text()[:256]} for path in paths]
            except (OSError, ValueError) as error:
                result['temporary_evidence_error'] = repr(error)
            if app and app.poll() is None: app.terminate(); app.wait(timeout=5)
            log.seek(0);result['native_log']=log.read();log.close()
            if previous.returncode==0:
                subprocess.run(['xdotool','windowactivate',previous.stdout.strip()],env=env,stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL)
    result['samples']=series;result['memory_samples']=checkpoints
    for launch in launches:
        own_samples = [sample for sample in series if sample['launch'] == launch['launch']]
        launch['sample_count'] = len(own_samples)
        if own_samples: launch['first_sample_delay_seconds'] = own_samples[0]['t']-launch['t']
        else: sampling_errors.append({'launch': launch['launch'], 'error': 'No memory samples for launched process'})
    result['launches']=launches;result['sampling_errors']=sampling_errors
    result['sampling_lifecycle_events']=sampling_lifecycle_events
    for observer in result['sampler_cpu']:
        if observer['cpu_seconds'] is None:
            sampling_errors.append({'launch': observer['launch'], 'error': 'Sampler thread did not complete CPU accounting'})
    try:
        result['app_cpu'] = cpu_summary(series, os.sysconf('SC_CLK_TCK'))
    except (ValueError, KeyError) as error:
        result['app_cpu'] = {'available': False, 'error': repr(error)}
        sampling_errors.append({'error': 'Invalid CPU accounting: '+repr(error)})
    result['observer_cpu'] = {
        'sampler_thread_seconds': sum(item['cpu_seconds'] for item in result['sampler_cpu']
                                      if item['cpu_seconds'] is not None),
        'limits': 'Sampler threads only, including proc reads and wakeups. Excludes main probe work, screenshots and external accounting subprocesses; not added to app CPU.'}
    result['memory_budget']=memory_budget([*series, *checkpoints.values()], sampling_errors)
    result['external_after']=external_series(env, 'after')
    external = [*result['external_before'], *result.get('external_checkpoints', {}).values(), *result['external_after']]
    result['external_accounting'] = {
        'all_snapshots_available': all(record['available'] for record in external),
        'snapshot_count': len(external),
        'physical_bytes_fully_attributed': False,
        'remaining_limit': 'XRes byte estimates omit glyph and other server allocation costs; external process/DRM deltas contain unrelated desktop activity.'}
    # Incomplete functional/external accounting evidence must never authorize a release.
    result['acceptance_passed']=False
    result['acceptance_pending']=['native pointer/scrollbar and wrapping interaction checks, screenshot review', 'review external resource estimates and unquantified glyph costs', 'native IME/accessibility and render/pointer latency evidence']
    output.write_text(json.dumps(result,indent=2)+'\n')
    print(json.dumps({k:result[k] for k in ('memory_budget','checks','acceptance_passed')},indent=2))
    if 'error' in result: print(result['error'])
    raise SystemExit(1)


if __name__ == '__main__': main()
