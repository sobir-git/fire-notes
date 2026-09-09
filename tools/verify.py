#!/usr/bin/env python3
"""Collect virtual native evidence sequentially; independent review remains required.

--assemble-existing accepts a JSON object with paths named memory (benchmark.json aggregate),
native_input, interactions (directory containing notes/, tab-menu/, inspection/),
and pointer_latency. Relative paths resolve against that JSON's directory.
Existing captures are validated and copied, never rewritten as fresh measurements.
"""
import argparse
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys

import release


def reference(path, directory):
    return {'path': os.path.relpath(path, directory), 'sha256': release.sha256(path)}


def write_json(path, value):
    path.write_text(json.dumps(value, indent=2) + '\n')


def aggregate_interactions(captures, output, binary_hash, sources):
    """Bind actual successful probe receipts; this function does not invent outcomes."""
    captures, output = Path(captures).resolve(), Path(output).resolve()
    output.mkdir()
    copied = output / 'captures'
    shutil.copytree(captures, copied)
    for bad in copied.rglob('failure*'):
        raise release.Rejected(f'failed interaction capture present: {bad}')
    source_dir = output / 'sources'
    source_dir.mkdir()
    source_refs = {}
    for name in ('notes_probe.py', 'tab_menu_probe.py', 'inspection_probe.py', 'private_session.py'):
        shutil.copyfile(sources / name, source_dir / name)
        source_refs[name] = reference(source_dir / name, output)
    notes_log = copied / 'notes-runner.log'
    if not notes_log.exists():
        notes_log = copied / 'notes/runner.log'
    receipt_paths = {'notes': copied/'notes/metrics.json', 'tabs': copied/'tab-menu/result.json',
                     'inspection': copied/'inspection/results.json', 'notes_completion': notes_log,
                     'session': copied/'notes/session.json', 'tree': copied/'inspection/tree.json'}
    screenshots = {}
    for name, subdir in [('notes', 'notes'), ('tabs', 'tab-menu'), ('inspection', 'inspection')]:
        screenshots[name] = {
            path.stem: reference(path, output)
            for path in sorted((copied / subdir).glob('*.png'))
        }
    report = {'schema': 1, 'producer': 'notes-native-probes', 'binary_sha256': binary_hash,
              'temporary_data': True,
              'source_receipt_note': 'Sources copied at assembly; capture provenance requires independent review.',
              'receipts': {name: reference(path, output) for name, path in receipt_paths.items()},
              'sources': source_refs,
              'screenshots': screenshots}
    release.validate_interactions(report, output, {})
    write_json(output/'results.json', report)
    return output/'results.json'


def copy_report(path, output):
    """Keep each producer's relative screenshot/raw-data references intact."""
    path = Path(path).resolve()
    shutil.copytree(path.parent, output)
    return output/path.name


def assemble(binary, output, paths):
    binary_hash = release.sha256(binary)
    reports = {}
    for name in ('memory', 'native_input', 'pointer_latency'):
        reports[name] = copy_report(paths[name], output/name)
    reports['interactions'] = aggregate_interactions(paths['interactions'], output/'interactions',
                                                     binary_hash, Path(__file__).resolve().parent)
    references = {name: reference(path, output) for name, path in reports.items()}
    manifest = {'schema': 1, 'binary_sha256': binary_hash, 'reports': references}
    for name, path in reports.items():
        report = json.loads(path.read_text())
        release.require(report.get('binary_sha256') == binary_hash, f'stale {name} binary hash')
        release.VALIDATORS[name](report, path.parent, manifest)
    pending = {'schema': 1, 'binary_sha256': binary_hash, 'status': 'pending',
               'reviewer': '', 'reviewed_at': '',
               'report_hashes': {name: ref['sha256'] for name, ref in references.items()},
               'unresolved': ['Independent visual, public API and external-cost/growth evidence review required.']}
    write_json(output/'review.json', pending)
    manifest['reports']['review'] = reference(output/'review.json', output)
    write_json(output/'manifest.json', manifest)
    return manifest


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary', required=True)
    parser.add_argument('--output', required=True, help='Fresh output directory')
    parser.add_argument('--assemble-existing', type=Path, help='JSON paths to existing captures; no native input')
    args = parser.parse_args(argv)
    binary = Path(args.binary).resolve()
    output = Path(args.output).resolve()
    output.mkdir(parents=True, exist_ok=False)
    scripts = Path(__file__).resolve().parent
    initial_hash = release.sha256(binary)
    if args.assemble_existing:
        config = args.assemble_existing.resolve()
        paths = {key: (config.parent/value).resolve() for key, value in json.loads(config.read_text()).items()}
    else:
        raw = output/'raw'
        raw.mkdir()
        def run(name, script, destination, *extra):
            with (raw/(name+'-runner.log')).open('w') as log:
                subprocess.run([sys.executable, str(script), '--binary', str(binary), '--output', str(destination), *extra],
                               check=True, stdout=log, stderr=subprocess.STDOUT)
            release.require(release.sha256(binary) == initial_hash, 'binary changed during native capture')
        run('benchmark', scripts/'benchmark.py', raw/'benchmark', '--growth')
        run('latency', scripts/'pointer_latency_probe.py', raw/'latency')
        run('native-input', scripts.parents[1]/'fire-ui/tools/linux_input_probe.py', raw/'native-input', '--target', 'notes')
        captures = raw/'interactions'
        captures.mkdir()
        for name, script in [('notes', 'notes_probe.py'), ('tab-menu', 'tab_menu_probe.py'), ('inspection', 'inspection_probe.py')]:
            run(name, scripts/script, captures/name)
            shutil.move(raw/(name+'-runner.log'), captures/(name+'-runner.log'))
        paths = {'memory': raw/'benchmark/benchmark.json',
                 'native_input': raw/'native-input/results.json', 'pointer_latency': raw/'latency/results.json',
                 'interactions': captures}
    assemble(binary, output, paths)
    release.require(release.sha256(binary) == initial_hash, 'binary changed during assembly')
    print(json.dumps({'collected': True, 'accepted': False, 'review': 'pending',
                      'manifest': str(output/'manifest.json'), 'binary_sha256': initial_hash}, indent=2))
    return 0


if __name__ == '__main__':
    try:
        raise SystemExit(main())
    except (OSError, ValueError, KeyError, subprocess.CalledProcessError) as error:
        print(json.dumps({'collected': False, 'accepted': False, 'error': str(error)}))
        raise SystemExit(1)
