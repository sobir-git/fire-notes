#!/usr/bin/env python3
"""Validate hash-bound native acceptance evidence, then optionally install one binary.

Manifest schema 1 contains binary_sha256 and reports named memory, native_input,
interactions, pointer_latency, review. Every report reference is {path, sha256},
relative to the manifest. Reports themselves must name the same binary_sha256.
Missing evidence rejects installation. This command never launches the app.
"""
import argparse
import hashlib
import json
import math
import os
from pathlib import Path
import shutil
import tempfile

from memory_policy import LIMIT_BYTES
WINDOWS = [[600, 400], [900, 600], [1200, 800]]
TYPED_TEXT = 'typing memory workload. ' * 13
MEMORY_CHECKS = ('typing', 'animated_fire', 'clipboard_paste', 'undo_paste', 'redo_paste',
                 'undo_redone_paste', 'tabs_visited', 'resize_geometry', 'scrolling',
                 'quiet_idle', 'restored_session')
INPUT_CHECKS = ('ibus_cangjie', 'ibus_selection', 'ibus_window_focus', 'ibus_focus',
                'ibus_external_set_value', 'ibus_external_replace_selected_text',
                'ibus_external_atspi_set_text_contents', 'atspi_unicode',
                'orca_editor_focus', 'orca_caret_navigation', 'orca_insertion',
                'orca_deletion', 'orca_selection', 'orca_button_focus',
                'orca_tab_navigation', 'orca_button_activation')
VISUAL_STATES = ('startup', 'typed_fire', 'selected_fire', 'scrolled', 'resized', 'restored', 'unicode')
EXTERNAL_FINDINGS = ('process_resident_and_swap', 'xres_backing_pixmaps', 'glyph_storage',
                     'drm_shared_buffers', 'growth')


class Rejected(ValueError):
    pass


def require(condition, message):
    if not condition:
        raise Rejected(message)


def sha256(path):
    digest = hashlib.sha256()
    with Path(path).open('rb') as file:
        for chunk in iter(lambda: file.read(1024 * 1024), b''):
            digest.update(chunk)
    return digest.hexdigest()


def number(value):
    return type(value) in (int, float) and math.isfinite(value)


def checked_file(reference, directory):
    require(isinstance(reference, dict), 'artifact reference must contain path and sha256')
    path = (directory / reference['path']).resolve()
    require(path.is_file(), f'missing artifact: {path}')
    require(sha256(path) == reference.get('sha256'), f'artifact hash mismatch: {path}')
    return path


def checked_report(reference, directory, binary_hash):
    path = checked_file(reference, directory)
    report = json.loads(path.read_text())
    require(report.get('binary_sha256') == binary_hash, f'stale binary hash in {path}')
    require(not report.get('error'), f'failed report {path}: {report.get("error")}')
    return report, path.parent


def validate_samples(samples, enforce_limit=True):
    require(isinstance(samples, list) and samples, 'no raw memory samples')
    fields = ('Private_Dirty', 'Private_Clean', 'Shared_Dirty', 'Shared_Clean', 'Rss', 'Pss', 'Swap', 'SwapPss')
    for index, sample in enumerate(samples):
        require(all(type(sample.get(field)) is int and sample[field] >= 0 for field in fields),
                f'invalid memory accounting in sample {index}')
        require(sample['Swap'] == 0 and sample['SwapPss'] == 0, f'swap in sample {index}')
        dirty = sample['Private_Dirty'] * 1024
        if enforce_limit:
            require(dirty < LIMIT_BYTES, f'sample {index}: {dirty} private dirty bytes; required < {LIMIT_BYTES}')
    return max(sample['Private_Dirty'] * 1024 for sample in samples)


def validate_external(report):
    before, after = report['external_before'], report['external_after']
    require(len(before) >= 3 and len(after) >= 3, 'three external before/after snapshots required')
    during = report['external_checkpoints']
    require({'startup', 'selected_fire', 'clipboard_paste', 'restored'} <= during.keys(),
            'external snapshots missing native workload states')
    for snapshot in [*before, *during.values(), *after]:
        require(snapshot.get('available') is True, 'external accounting unavailable')
        data = snapshot['accounting']
        require(not data['query_errors'], 'XRes query error')
        require({item['pid'] for item in data['processes']} == set(snapshot['requested_pids']),
                'external process coverage incomplete')
        for process in data['processes']:
            require(not any(key in process for key in ('memory_error', 'drm_error', 'fd_errors')),
                    'external process accounting error')
            memory = process['memory_bytes']
            require(all(type(memory.get(key)) is int and memory[key] >= 0
                        for key in ('Rss', 'Pss', 'Private_Clean', 'Private_Dirty', 'Shared_Clean', 'Shared_Dirty', 'Swap', 'SwapPss')),
                    'external memory fields missing')
            # External swap can predate the test. It is reviewed alongside resident
            # changes; it must never be silently replaced by zero or added to RSS.
            require(isinstance(process['drm_clients'], dict), 'DRM accounting missing')
        if snapshot['app_pid'] is not None:
            clients = [item for item in data['xres_clients'] if item['pid'] == snapshot['app_pid']]
            require(clients, 'app XRes resources missing')
            for client in clients:
                    require(type(client.get('pixmap_bytes_estimate')) is int
                        and isinstance(client.get('resource_counts'), dict)
                        and isinstance(client.get('resource_bytes_estimates'), list), 'incomplete app XRes evidence')


def validate_runtime_costs(report):
    from memory_probe import cpu_summary
    environment = report['launch_environment']
    require(environment.get('allocator_environment') == {}
            and environment.get('allocator_environment_is_default') is True,
            'default release must be measured without allocator or preload overrides')
    cpu = report['app_cpu']
    require(cpu.get('available') is True and type(cpu.get('ticks_per_second')) is int
            and cpu['ticks_per_second'] > 0, 'process CPU accounting missing')
    samples = report['samples']
    require(all(type(sample.get('cpu_ticks')) is int and sample['cpu_ticks'] >= 0
                for sample in samples), 'raw process CPU counters missing')
    actual = cpu_summary(samples, cpu['ticks_per_second'])
    require(all(cpu.get(key) == actual[key] for key in
                ('sampled_process_cpu_seconds', 'observed_process_seconds', 'launches', 'stages')),
            'reported CPU differs from raw samples')
    observers = report['sampler_cpu']
    require(len(observers) == len(report['launches'])
            and all(number(item.get('cpu_seconds')) and item['cpu_seconds'] >= 0 for item in observers),
            'sampler CPU overhead missing')


def validate_memory(report, _directory, _manifest):
    require(report.get('units') == 'KiB', 'memory report units must be KiB')
    require(report.get('app_args') == [], 'ordinary memory workload cannot select reduced or diagnostic app arguments')
    workload = report['workload']
    require(workload.get('tabs') == 3 and workload.get('document_bytes') == 8192
            and workload.get('startup_only') is False, 'frozen ordinary workload is three 8192-byte documents')
    require(workload.get('logical_window_sizes') == WINDOWS and workload.get('typed_text') == TYPED_TEXT,
            'window or typing workload differs from frozen acceptance')
    scale = workload['observed_scale']
    require(number(scale) and scale > 0, 'invalid display scale')
    require(workload['physical_window_sizes'] == [[round(w*scale), round(h*scale)] for w,h in WINDOWS],
            'physical window sizes do not match the logical workload')
    require(not report.get('sampling_errors'), 'live memory sampling errors')
    checkpoints = report['memory_samples']
    needed = {'startup', 'typed_fire', 'selected_fire', 'clipboard_paste', 'tabs', 'scrolled', 'restored'}
    needed |= {f'resize_logical_{w}x{h}' for w,h in WINDOWS}
    require(needed <= checkpoints.keys(), 'missing native memory checkpoints')
    peak = validate_samples([*report['samples'], *checkpoints.values()])
    require(report['memory_budget']['max_sampled_bytes'] == peak, 'reported peak differs from raw samples')
    launches = report['launches']
    require(len(launches) == 2, 'fresh launch and restored launch required')
    for launch in launches:
        require(number(launch['first_sample_delay_seconds']) and 0 <= launch['first_sample_delay_seconds'] <= .125,
                'startup sampling began too late')
        samples = [sample for sample in report['samples'] if sample.get('launch') == launch['launch']]
        require(len(samples) == launch['sample_count'] and len(samples) >= 2, 'launch sampling incomplete')
        require(all(sample.get('pid') == launch['pid'] for sample in samples), 'sample PID/launch mismatch')
        times = [sample['t'] for sample in samples]
        require(all(number(value) for value in times) and all(b >= a for a,b in zip(times, times[1:])),
                'invalid sample timestamps')
        require(max(b-a for a,b in zip(times, times[1:])) <= .125, 'memory sampling gap exceeds 125ms')
    require(all(report['checks'].get(key) is True for key in MEMORY_CHECKS), 'native workload assertion missing')
    for key in ('typing', 'clipboard_paste', 'undo_paste', 'redo_paste', 'undo_redone_paste'):
        evidence = report['document_checks'][key]
        require(evidence['actual_sha256'] == evidence['expected_sha256']
                and evidence['actual_bytes'] == evidence['expected_bytes'], f'{key} document differs')
        expected_bytes = (8192+len(TYPED_TEXT.encode())) * (2 if key in ('clipboard_paste', 'redo_paste') else 1)
        require(evidence['actual_bytes'] == expected_bytes, f'{key} document workload shrank')
    typing = report['typing_injection']
    require(typing['backend'] == 'XTest' and typing['delivered_events'] == len(TYPED_TEXT)
            and len(typing['event_timings']) == len(TYPED_TEXT), 'full native typing was not delivered')
    require(len({visit['active'] for visit in report['tab_visits']}) == 3, 'not all tabs visited')
    require(report['scrolled_view']['y'] > 0, 'scrolling outcome missing')
    idle = report['idle']
    require(number(idle['elapsed_seconds']) and idle['elapsed_seconds'] >= 2
            and type(idle['cpu_ticks']) is int and 0 <= idle['cpu_ticks'] <= 2, 'quiet idle requirement failed')
    for field in ('tabs', 'active', 'views', 'width', 'height', 'titles'):
        require(report['session_before_restart'][field] == report['session_after_restart'][field],
                f'restored {field} differs')
    display = report['display_state']
    require(display.get('kind') == 'private' and display.get('available') is True
            and display.get('isolated') is True and display.get('provider') == 'Xvfb',
            'acceptance requires the isolated native Xvfb workload')
    context = report['run_context']
    require(context.get('accessibility_bus') == 'private_enabled' and context.get('software_gl_override') is False,
            'native accessibility must be enabled without graphics overrides')
    validate_runtime_costs(report)
    validate_external(report)


def validate_memory_aggregate(report, directory, manifest):
    require(report.get('schema') == 1, 'benchmark schema 1 required')
    ordinary = report['ordinary']
    require(isinstance(ordinary, list) and len(ordinary) == 3, 'three fresh ordinary runs required')
    require(len({reference['sha256'] for reference in ordinary}) == 3,
            'ordinary runs must be distinct reports')
    for reference in ordinary:
        raw, source = checked_report(reference, directory, report['binary_sha256'])
        validate_memory(raw, source, manifest)
    growth = report['growth']
    from benchmark import GROWTH, assess
    require({(entry['scope'], entry['document_bytes'], entry['tabs']) for entry in growth} >= set(GROWTH),
            'growth coverage incomplete: startup sizes and active editing workloads are required')
    for entry in growth:
        raw, _ = checked_report(entry['report'], directory, report['binary_sha256'])
        require(raw['workload']['document_bytes'] == entry['document_bytes']
                and raw['workload']['tabs'] == entry['tabs'], 'growth workload differs from its raw report')
        require(raw.get('units') == 'KiB' and not raw.get('sampling_errors'), 'growth sampling invalid')
        validate_samples(raw['samples'], enforce_limit=False)
        failures = assess(raw, report['binary_sha256'], entry['document_bytes'], entry['tabs'], entry['scope'])
        require(not failures, 'growth workload failed: '+', '.join(failures))
        validate_runtime_costs(raw)


def validate_input(report, directory, _manifest):
    require(report.get('target') == 'notes' and report.get('passed') is True, 'native input report did not pass for Notes')
    require(all(report['checks'].get(key) for key in INPUT_CHECKS), 'missing IME/AT-SPI/Orca assertions')
    for name in ('set_value', 'replace_selected_text', 'atspi_set_text_contents'):
        check = report['checks']['ibus_external_'+name]
        require(check == {'after_edit': 'replacement', 'composition': None, 'after_commit_key': 'replacement '},
                'stale composition survived external editing')
    for name in ('orca_editor_focus', 'orca_caret_navigation', 'orca_insertion', 'orca_deletion',
                 'orca_selection', 'orca_button_focus', 'orca_tab_navigation'):
        require(isinstance(report['checks'][name], list) and all('SPEECH OUTPUT:' in row for row in report['checks'][name]),
                'Orca speech evidence missing')
    for name in ('ime-preedit.json', 'ime-preedit.png', 'orca-speech.log'):
        checked_file(report['artifacts'][name], directory)


def validate_interactions(report, directory, _manifest):
    require(report.get('schema') == 1 and report.get('producer') == 'notes-native-probes',
            'actual native probe aggregate required')
    require(report.get('temporary_data') is True, 'interaction probes require temporary data')
    binary_hash = report['binary_sha256']
    receipts = report['receipts']
    notes, _ = checked_report(receipts['notes'], directory, binary_hash)
    tabs, _ = checked_report(receipts['tabs'], directory, binary_hash)
    inspection, _ = checked_report(receipts['inspection'], directory, binary_hash)
    log = checked_file(receipts['notes_completion'], directory).read_text()
    require('PASS: original layout, animated fire, typing, clipboard, undo/redo, rename, tabs, search, commands, native file opening, raw Markdown, wrap, scrollbar, resize, idle, session restoration and save-before-close.' in log
            and 'Traceback (most recent call last)' not in log, 'full Notes probe did not complete')
    require(notes.get('idle_seconds', 0) >= 2 and type(notes.get('idle_cpu_ticks')) is int
            and 0 <= notes['idle_cpu_ticks'] <= 2, 'Notes quiet idle failed')
    required_tabs = {'inactive tab target', 'wrap', 'focus restoration', 'rename', 'save',
                     'close preserves files', 'last-tab disabled', 'outside dismiss',
                     'hover over text and padding', 'trash latest input', 'restore title and view',
                     'last-tab trash', 'close waits for trash', '30-day cleanup'}
    require(isinstance(tabs.get('passed'), list) and required_tabs <= set(tabs['passed']),
            'tab context probe assertions missing')
    for key, value in {'semantic_editing': 'passed', 'native_keyboard_undo': 'passed',
                       'invalid_and_stale_requests': 'rejected', 'socket_cleanup': 'passed',
                       'temporary_data': True}.items():
        require(inspection.get(key) == value, f'inspection {key} failed')
    idle = inspection.get('idle_cpu_ticks_over_2_seconds')
    require(type(idle) is int and 0 <= idle <= 2, 'inspection quiet idle failed')
    session = json.loads(checked_file(receipts['session'], directory).read_text())
    require(isinstance(session.get('tabs'), list) and len(session['tabs']) >= 2
            and session.get('active') in session['tabs'] and isinstance(session.get('views'), dict),
            'saved native session receipt missing')
    tree = json.loads(checked_file(receipts['tree'], directory).read_text())
    require(tree.get('protocol') == 1 and any(node.get('key') == 'note-body' for node in tree['nodes']),
            'native semantic tree missing')
    required_images = {
        'notes': ('01-empty', '01a-discarded', '02-writing', '03-fire-selection',
                  '04-fire-next-frame', '05-rename', '06-picker', '07-commands',
                  '07a-empty-commands', '07b-narrow-commands', '07c-empty-notes',
                  '08-imported', '09-narrow', '10-scrollbar', '11-restored'),
        'tabs': ('00-hover-padding', '00-hover-text', '01-actions', '02-checked-wrap',
                 '03-renamed', '04-last-tab', '05-trash', '06-restored-note', '07-empty-trash'),
        'inspection': ('bidi-selection',),
    }
    from PIL import Image
    for group, names in required_images.items():
        for name in names:
            with Image.open(checked_file(report['screenshots'][group][name], directory)) as image:
                image.verify()
    for name in ('notes_probe.py', 'tab_menu_probe.py', 'inspection_probe.py', 'private_session.py'):
        require(checked_file(report['sources'][name], directory).name == name,
                'probe source receipt missing')


def validate_latency(report, directory, _manifest):
    require(report.get('schema') == 1 and report.get('display_kind') in ('physical_x11', 'private_x11'), 'native drawable latency schema 1 required')
    require(report.get('metric') == 'native_input_to_drawable_pixels', 'command acknowledgement is not observed drawable response')
    require(report.get('complete') is True, 'latency workload did not complete')
    desktop = report.get('desktop_state', {})
    if report['display_kind'] == 'physical_x11':
        require(desktop.get('available') is True and desktop.get('locked') is False, 'physical latency desktop must be verified unlocked')
    else:
        require(desktop.get('kind') == 'private_x11' and report.get('temporary_data') is True, 'virtual latency requires isolated display and temporary data')
    for name in ('pointer_idle', 'pointer_active_fire', 'resize'):
        samples = report['samples'][name]
        require(len(samples) >= 20, f'{name}: at least 20 observed frame samples required')
        delays = []
        for sample in samples:
            expected_input = 'XResizeWindow' if name == 'resize' else 'XTestFakeMotionEvent'
            require(sample.get('input_kind') == expected_input, 'latency native request kind missing')
            require(number(sample['milliseconds']) and sample['milliseconds'] >= 0, 'invalid latency')
            require(type(sample['input_monotonic_ns']) is int and type(sample['observed_monotonic_ns']) is int,
                    'raw input/observation timestamps missing')
            elapsed_ms = (sample['observed_monotonic_ns']-sample['input_monotonic_ns']) / 1_000_000
            require(abs(elapsed_ms-sample['milliseconds']) < .001, 'latency differs from raw timestamps')
            expected = sample['expected_pixels']
            require(isinstance(expected, list) and expected and expected == sample['observed_pixels'], 'target pixels not verified')
            from PIL import Image
            with Image.open(checked_file(sample['frame'], directory)) as after, Image.open(checked_file(sample['before_frame'], directory)) as before:
                after_rgb, before_rgb = after.convert('RGB'), before.convert('RGB')
                changed = False
                for x, y, color in expected:
                    require(type(x) is int and type(y) is int and isinstance(color, list) and len(color) == 3,
                            'pixel evidence must contain x, y and RGB')
                    require(list(after_rgb.getpixel((x,y))) == color, 'captured pixels differ from expected response')
                    changed |= list(before_rgb.getpixel((x,y))) != color
                require(changed, 'latency target pixels were already present before input')
            delays.append(sample['milliseconds'])
        delays.sort()
        require(delays[math.ceil(.95*len(delays))-1] <= 50 and max(delays) <= 150,
                f'{name}: drawable response must stay within 50ms p95 and 150ms maximum')
    require(isinstance(report.get('measurement_limits'), list) and report['measurement_limits'], 'latency method limitations missing')


def validate_review(report, directory, manifest):
    require(report.get('schema') == 1 and isinstance(report.get('reviewer'), str) and report['reviewer'].strip(),
            'named evidence review required')
    require(isinstance(report.get('reviewed_at'), str) and report['reviewed_at'].strip(), 'review date missing')
    require(report['report_hashes'] == {name: ref['sha256'] for name,ref in manifest['reports'].items() if name != 'review'},
            'review does not bind the supplied reports')
    require(report.get('unresolved') == [], 'review has unresolved acceptance findings')
    from PIL import Image
    for state in VISUAL_STATES:
        visual = report['visuals'][state]
        path = checked_file(visual['image'], directory)
        with Image.open(path) as image:
            require(list(image.size) == visual['physical_size'], 'reviewed image dimensions differ')
            image.verify()
        require(isinstance(visual.get('observations'), str) and len(visual['observations'].strip()) >= 20,
                f'{state}: visual observations missing')
        require(visual.get('accepted') is True, f'{state}: visual review rejected')
    for name in EXTERNAL_FINDINGS:
        finding = report['external'][name]
        require(isinstance(finding.get('method'), str) and len(finding['method'].strip()) >= 20
                and isinstance(finding.get('observations'), str) and len(finding['observations'].strip()) >= 20,
                f'{name}: measured external-cost review missing')
        require(finding['evidence'], f'{name}: raw external evidence missing')
        for reference in finding['evidence']: checked_file(reference, directory)
    growth = report['growth']
    require({sample['document_bytes'] for sample in growth} >= {1024, 8192, 65536, 262144, 1048576}
            and {sample['tabs'] for sample in growth} >= {1, 3, 10}, 'document/tab growth coverage incomplete')
    for sample in growth:
        raw, _ = checked_report(sample['report'], directory, report['binary_sha256'])
        require(raw['workload']['document_bytes'] == sample['document_bytes'] and raw['workload']['tabs'] == sample['tabs'],
                'growth workload metadata differs from raw report')
        require(not raw.get('sampling_errors') and raw.get('units') == 'KiB', 'growth accounting invalid')
        # Larger workloads are measured and reviewed explicitly, never represented
        # as a pass of the fixed ordinary workload or excluded from published evidence.
        validate_samples(raw['samples'], enforce_limit=False)


VALIDATORS = {'memory': validate_memory_aggregate, 'native_input': validate_input,
              'interactions': validate_interactions, 'pointer_latency': validate_latency, 'review': validate_review}


def validate(binary, manifest_path):
    binary_hash = sha256(binary)
    manifest_path = Path(manifest_path).resolve()
    manifest = json.loads(manifest_path.read_text())
    require(manifest.get('schema') == 1, 'acceptance manifest schema 1 required')
    require(manifest.get('binary_sha256') == binary_hash, 'manifest binary hash is stale')
    require(set(manifest.get('reports', {})) == set(VALIDATORS), 'all five acceptance reports are required')
    failures = []
    for name, validator in VALIDATORS.items():
        try:
            report, directory = checked_report(manifest['reports'][name], manifest_path.parent, binary_hash)
            validator(report, directory, manifest)
        except (OSError, ValueError, KeyError, TypeError, IndexError, AttributeError) as error:
            failures.append(f'{name}: {error}')
    require(not failures, '\n'.join(failures))
    return {'accepted': True, 'binary_sha256': binary_hash, 'manifest_sha256': sha256(manifest_path)}


def install(binary, destination, expected_hash):
    destination = Path(destination).absolute()
    require(not destination.is_symlink(), 'refusing to replace a symlink')
    require(not destination.exists() or (destination.is_file() and os.access(destination, os.X_OK)),
            'refusing to overwrite an existing non-executable file')
    destination.parent.mkdir(parents=True, exist_ok=True)
    temporary = None
    try:
        with tempfile.NamedTemporaryFile(prefix='.fire-notes-install-', dir=destination.parent, delete=False) as file:
            temporary = Path(file.name)
            with Path(binary).open('rb') as source: shutil.copyfileobj(source, file)
            file.flush(); os.fsync(file.fileno())
        require(sha256(temporary) == expected_hash, 'binary changed after acceptance validation')
        temporary.chmod(0o755)
        os.replace(temporary, destination)
    finally:
        if temporary is not None and temporary.exists(): temporary.unlink()


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary', required=True)
    parser.add_argument('--manifest', required=True)
    parser.add_argument('--install-to', help='Explicit executable destination; installation happens only after every check passes')
    args = parser.parse_args(argv)
    try:
        result = validate(args.binary, args.manifest)
        if args.install_to:
            install(args.binary, args.install_to, result['binary_sha256'])
            result['installed_to'] = str(Path(args.install_to).absolute())
        print(json.dumps(result, indent=2))
        return 0
    except (OSError, ValueError, KeyError, TypeError, IndexError, ImportError, AttributeError) as error:
        print(json.dumps({'accepted': False, 'error': str(error)}, indent=2))
        return 1


if __name__ == '__main__':
    raise SystemExit(main())
