"""Synthetic rejection fixtures test plumbing; they are never native evidence."""
import json
from pathlib import Path
import tempfile
import unittest
from PIL import Image

import release
import verify


class InteractionEvidenceTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.captures = self.root/'captures'
        for name in ('notes', 'tab-menu', 'inspection'):
            (self.captures/name).mkdir(parents=True)
        self.binary_hash = 'synthetic-unit-test-only'
        def write(name, value):
            (self.captures/name).write_text(json.dumps(value))
        write('notes/metrics.json', {'binary_sha256': self.binary_hash, 'idle_seconds': 2, 'idle_cpu_ticks': 0})
        write('notes/session.json', {'tabs': ['a', 'b'], 'active': 'a', 'views': {}})
        write('tab-menu/result.json', {'binary_sha256': self.binary_hash, 'passed': [
            'inactive tab target', 'wrap', 'focus restoration', 'rename', 'save', 'close preserves files',
            'last-tab disabled', 'outside dismiss', 'hover over text and padding', 'trash latest input',
            'restore title and view', 'last-tab trash', 'close waits for trash', '30-day cleanup']})
        write('inspection/results.json', {'binary_sha256': self.binary_hash, 'semantic_editing': 'passed',
              'native_keyboard_undo': 'passed', 'invalid_and_stale_requests': 'rejected',
              'socket_cleanup': 'passed', 'temporary_data': True, 'idle_cpu_ticks_over_2_seconds': 0})
        write('inspection/tree.json', {'protocol': 1, 'nodes': [{'key': 'note-body'}]})
        (self.captures/'notes-runner.log').write_text('PASS: original layout, animated fire, typing, clipboard, undo/redo, rename, tabs, search, commands, native file opening, raw Markdown, wrap, scrollbar, resize, idle, session restoration and save-before-close.')
        for directory, names in {
            'notes': '01-empty 01a-discarded 02-writing 03-fire-selection 04-fire-next-frame 05-rename 06-picker 07-commands 07a-empty-commands 07b-narrow-commands 07c-empty-notes 08-imported 09-narrow 10-scrollbar 11-restored',
            'tab-menu': '00-hover-padding 00-hover-text 01-actions 02-checked-wrap 03-renamed 04-last-tab 05-trash 06-restored-note 07-empty-trash',
            'inspection': 'bidi-selection',
        }.items():
            for name in names.split():
                Image.new('RGB', (2, 2)).save(self.captures/directory/(name+'.png'))

    def assemble(self):
        return verify.aggregate_interactions(self.captures, self.root/'aggregate', self.binary_hash, Path(__file__).parent)

    def test_actual_receipts_aggregate_and_tampering_rejected(self):
        path = self.assemble()
        report = json.loads(path.read_text())
        release.validate_interactions(report, path.parent, {})
        image = path.parent/report['screenshots']['inspection']['bidi-selection']['path']
        image.write_bytes(b'changed')
        with self.assertRaisesRegex(release.Rejected, 'hash mismatch'):
            release.validate_interactions(report, path.parent, {})

    def test_stale_binary_rejected(self):
        path = self.captures/'notes/metrics.json'
        value = json.loads(path.read_text()); value['binary_sha256'] = 'older'
        path.write_text(json.dumps(value))
        with self.assertRaisesRegex(release.Rejected, 'stale binary'):
            self.assemble()

    def test_missing_native_assertion_rejected(self):
        path = self.captures/'tab-menu/result.json'
        value = json.loads(path.read_text()); value['passed'].remove('30-day cleanup')
        path.write_text(json.dumps(value))
        with self.assertRaisesRegex(release.Rejected, 'assertions missing'):
            self.assemble()

    def test_incomplete_workflow_rejected(self):
        (self.captures/'notes-runner.log').write_text('Started but never finished')
        with self.assertRaisesRegex(release.Rejected, 'did not complete'):
            self.assemble()

    def test_failure_capture_rejected(self):
        (self.captures/'notes/failure.png').write_bytes(b'failed')
        with self.assertRaisesRegex(release.Rejected, 'failed interaction'):
            self.assemble()


if __name__ == '__main__':
    unittest.main()
