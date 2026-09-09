"""Gate rejection tests use synthetic unit data, never native acceptance evidence."""
import contextlib
import io
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch
import release


def sample(dirty=2929, **changes):
    result = dict.fromkeys(('Private_Dirty', 'Private_Clean', 'Shared_Dirty', 'Shared_Clean',
                           'Rss', 'Pss', 'Swap', 'SwapPss'), 0)
    result.update(Private_Dirty=dirty, Rss=dirty, Pss=dirty)
    result.update(changes)
    return result


class ReleaseTests(unittest.TestCase):
    def test_allocator_tuning_cannot_authorize_default_release(self):
        for environment in ({}, {'GLIBC_TUNABLES': 'glibc.malloc.arena_max=1'},
                            {'LD_PRELOAD': '/diagnostic/profiler.so'}):
            report = {'launch_environment': {'allocator_environment': environment,
                                              'allocator_environment_is_default': False}}
            with self.assertRaisesRegex(release.Rejected, 'without allocator'):
                release.validate_runtime_costs(report)

    def test_cpu_report_must_match_the_measured_counters(self):
        from memory_probe import cpu_summary
        samples = [dict(launch=0, pid=1, stage='typing', t=t, cpu_ticks=ticks)
                   for t, ticks in ((0, 0), (1, 10), (2, 20))]
        report = {'launch_environment': {'allocator_environment': {},
                                          'allocator_environment_is_default': True},
                  'samples': samples, 'app_cpu': cpu_summary(samples, 100),
                  'sampler_cpu': [{'cpu_seconds': .01}], 'launches': [{}]}
        release.validate_runtime_costs(report)
        report['app_cpu']['sampled_process_cpu_seconds'] = 0
        with self.assertRaisesRegex(release.Rejected, 'differs from raw'):
            release.validate_runtime_costs(report)

    def test_exact_byte_threshold(self):
        self.assertEqual(release.LIMIT_BYTES, 4_000_000)
        self.assertEqual(release.validate_samples([sample()]), 2_999_296)
        with self.assertRaisesRegex(release.Rejected, '4000768'):
            release.validate_samples([sample(3907)])

    def test_swap_and_swap_pss_each_reject(self):
        for key in ('Swap', 'SwapPss'):
            with self.subTest(key=key), self.assertRaisesRegex(release.Rejected, 'swap'):
                release.validate_samples([sample(**{key: 1})])

    def test_raw_accounting_required(self):
        for samples in ([], [dict(Private_Dirty=1)], [sample(Rss=True)], [sample(Pss=-1)]):
            with self.subTest(samples=samples), self.assertRaises(release.Rejected):
                release.validate_samples(samples)

    def test_growth_allows_resident_growth_but_never_swap(self):
        self.assertEqual(release.validate_samples([sample(10000)], False), 10240000)
        with self.assertRaises(release.Rejected):
            release.validate_samples([sample(10000, Swap=1)], False)

    def test_startup_growth_cannot_replace_large_editing(self):
        growth = [{'scope': 'startup', 'document_bytes': size, 'tabs': tabs}
                  for size, tabs in ((1024, 1), (8192, 3), (65536, 10), (262144, 1), (1048576, 1))]
        report = {'schema': 1, 'binary_sha256': 'binary',
                  'ordinary': [{'sha256': str(i)} for i in range(3)], 'growth': growth}
        with patch.object(release, 'checked_report', return_value=({}, Path('.'))), \
             patch.object(release, 'validate_memory'), \
             self.assertRaisesRegex(release.Rejected, 'active editing'):
            release.validate_memory_aggregate(report, Path('.'), {})

    def test_artifact_and_binary_hash_binding(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            report = root / 'report.json'
            report.write_text(json.dumps({'binary_sha256': 'old'}))
            reference = {'path': report.name, 'sha256': release.sha256(report)}
            with self.assertRaisesRegex(release.Rejected, 'stale binary'):
                release.checked_report(reference, root, 'new')
            report.write_text('{}')
            with self.assertRaisesRegex(release.Rejected, 'hash mismatch'):
                release.checked_file(reference, root)

    def test_missing_report_blocks_install_and_preserves_destination(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            binary, destination, manifest = root/'binary', root/'installed', root/'manifest.json'
            binary.write_bytes(b'unit test binary')
            destination.write_bytes(b'existing installed binary')
            manifest.write_text(json.dumps({'schema': 1, 'binary_sha256': release.sha256(binary), 'reports': {}}))
            with patch.object(release, 'install') as install, contextlib.redirect_stdout(io.StringIO()) as output:
                status = release.main(['--binary', str(binary), '--manifest', str(manifest), '--install-to', str(destination)])
            self.assertEqual(status, 1)
            self.assertFalse(json.loads(output.getvalue())['accepted'])
            install.assert_not_called()
            self.assertEqual(destination.read_bytes(), b'existing installed binary')

    def test_stale_manifest_rejected(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            binary, manifest = root/'binary', root/'manifest.json'
            binary.write_bytes(b'changed binary')
            manifest.write_text(json.dumps({'schema': 1, 'binary_sha256': 'stale', 'reports': {}}))
            with self.assertRaisesRegex(release.Rejected, 'stale'):
                release.validate(binary, manifest)

    def test_pass_boolean_cannot_override_excessive_raw_memory(self):
        report = {'accepted': True, 'acceptance_passed': True, 'units': 'KiB', 'app_args': [],
                  'workload': {'tabs': 3, 'document_bytes': 8192, 'startup_only': False,
                               'logical_window_sizes': release.WINDOWS, 'typed_text': release.TYPED_TEXT,
                               'observed_scale': 1, 'physical_window_sizes': release.WINDOWS},
                  'samples': [sample(4000)], 'memory_samples': {}}
        names = ['startup', 'typed_fire', 'selected_fire', 'clipboard_paste', 'tabs', 'scrolled', 'restored']
        names += [f'resize_logical_{w}x{h}' for w,h in release.WINDOWS]
        report['memory_samples'] = {name: sample() for name in names}
        with self.assertRaisesRegex(release.Rejected, '4096000'):
            release.validate_memory(report, None, None)

    def test_install_rechecks_binary_and_preserves_existing_file(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            binary, destination = root/'binary', root/'installed'
            binary.write_bytes(b'changed since validation')
            destination.write_bytes(b'existing executable')
            destination.chmod(0o700)
            with self.assertRaisesRegex(release.Rejected, 'changed after acceptance'):
                release.install(binary, destination, 'old hash')
            self.assertEqual(destination.read_bytes(), b'existing executable')
            self.assertEqual(list(root.glob('.fire-notes-install-*')), [])

    def test_boolean_interaction_claim_is_not_evidence(self):
        with self.assertRaisesRegex(release.Rejected, 'actual native probe aggregate'):
            release.validate_interactions({'schema': 1, 'checks': {'editing': {'expected': True, 'observed': True}}}, None, None)


if __name__ == '__main__':
    unittest.main()
