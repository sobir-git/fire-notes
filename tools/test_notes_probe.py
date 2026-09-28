"""Regression checks for the native Notes probe's own input automation."""
from pathlib import Path
import unittest

from notes_probe import paste_chooser_path
from unittest.mock import MagicMock, patch


class ChooserPathTests(unittest.TestCase):
    @patch('notes_probe.subprocess.Popen')
    @patch('notes_probe.time.sleep')
    def test_path_uses_gtk_clipboard_owner_without_xclip(self, _sleep, popen):
        owner = MagicMock()
        owner.stdout.readline.return_value = 'ready\n'
        popen.return_value = owner
        keys = []
        path = Path('/tmp/notes with spaces/external.md')

        paste_chooser_path(keys.append, {'DISPLAY': ':99'}, path)

        command = popen.call_args.args[0]
        self.assertEqual(command[:2], ['python3', '-c'])
        self.assertEqual(command[3], str(path))
        self.assertIn('Gtk.Clipboard.get', command[2])
        self.assertEqual(keys, ['ctrl+v'])
        owner.terminate.assert_called_once()
