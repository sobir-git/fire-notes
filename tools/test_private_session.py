import os
from pathlib import Path
import unittest
from unittest.mock import MagicMock, patch

from private_session import PrivateSession


class PrivateSessionTests(unittest.TestCase):
    def test_service_activation_uses_only_private_environment(self):
        original = {'DISPLAY': ':0', 'DBUS_SESSION_BUS_ADDRESS': 'unix:path=/user/bus',
                    'XDG_RUNTIME_DIR': '/user/runtime', 'HOME': '/user/home',
                    'WAYLAND_DISPLAY': 'wayland-0', 'AT_SPI_BUS_ADDRESS': 'unix:path=/user/atspi',
                    'LIBGL_ALWAYS_SOFTWARE': '1'}
        bus = MagicMock(pid=987654321)
        bus.stdout.readline.return_value = 'unix:path=/private/bus\n'
        with patch.dict(os.environ, original, clear=True), \
             patch('private_session.subprocess.Popen', return_value=bus) as spawn, \
             patch('private_session.subprocess.run') as run, \
             patch('private_session.os.killpg', side_effect=ProcessLookupError):
            with PrivateSession() as session:
                directory = session.directory
                env = session.activate(':99')
                self.assertEqual(dict(os.environ), original)
                self.assertEqual(env['DISPLAY'], ':99')
                self.assertEqual(env['DBUS_SESSION_BUS_ADDRESS'], 'unix:path=/private/bus')
                self.assertNotIn('WAYLAND_DISPLAY', env)
                self.assertNotIn('AT_SPI_BUS_ADDRESS', env)
                self.assertNotIn('LIBGL_ALWAYS_SOFTWARE', env)
                self.assertTrue(Path(env['HOME']).is_relative_to(directory))
                self.assertEqual(Path(env['XDG_RUNTIME_DIR']).stat().st_mode & 0o777, 0o700)
                self.assertNotIn('--systemd', run.call_args.args[0])
                self.assertEqual(run.call_args.kwargs['env']['DISPLAY'], ':99')
                self.assertEqual(spawn.call_args.kwargs['env']['DISPLAY'], ':99')
            self.assertFalse(directory.exists())


if __name__ == '__main__':
    unittest.main()
