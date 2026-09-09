"""Private desktop services for native probes; never connects to the user's bus."""
import os
from pathlib import Path
import signal
import subprocess
import tempfile
import time


class PrivateSession:
    def __enter__(self):
        self.temporary = tempfile.TemporaryDirectory(prefix='fire-notes-probe-')
        self.directory = Path(self.temporary.name)
        self.bus = None
        self.env = dict(os.environ)
        for key in ('DISPLAY', 'WAYLAND_DISPLAY', 'XAUTHORITY', 'DBUS_SESSION_BUS_ADDRESS',
                    'DBUS_STARTER_ADDRESS', 'DBUS_STARTER_BUS_TYPE', 'AT_SPI_BUS_ADDRESS',
                    'SESSION_MANAGER', 'GNOME_KEYRING_CONTROL', 'SSH_AUTH_SOCK',
                    'LIBGL_ALWAYS_SOFTWARE', 'XDG_SESSION_ID'):
            self.env.pop(key, None)
        for key, name in (('HOME', 'home'), ('XDG_DATA_HOME', 'data'),
                          ('XDG_CONFIG_HOME', 'config'), ('XDG_CACHE_HOME', 'cache'),
                          ('XDG_RUNTIME_DIR', 'runtime')):
            path = self.directory / name
            path.mkdir(mode=0o700)
            self.env[key] = str(path)
        self.env.update(WINIT_UNIX_BACKEND='x11', GDK_BACKEND='x11',
                        XDG_SESSION_TYPE='x11', XDG_CURRENT_DESKTOP='fire-probe')
        # A real GTK portal works without a GNOME shell on the private X server.
        config = self.directory / 'config/xdg-desktop-portal'
        config.mkdir()
        (config / 'portals.conf').write_text('[preferred]\ndefault=gtk\n')
        return self

    def activate(self, display):
        assert self.bus is None
        self.env['DISPLAY'] = display
        self.bus = subprocess.Popen(
            ['dbus-daemon', '--session', '--nofork', '--print-address=1'],
            env=self.env, stdout=subprocess.PIPE, text=True, start_new_session=True)
        address = self.bus.stdout.readline().strip()
        assert address.startswith('unix:'), 'Private session bus did not start'
        self.env['DBUS_SESSION_BUS_ADDRESS'] = address
        # No --systemd: updating the user's service manager would escape isolation.
        # https://dbus.freedesktop.org/doc/dbus-update-activation-environment.1.html
        subprocess.run(['dbus-update-activation-environment', 'DISPLAY', 'HOME',
                        'XDG_DATA_HOME', 'XDG_CONFIG_HOME', 'XDG_CACHE_HOME',
                        'XDG_RUNTIME_DIR', 'XDG_CURRENT_DESKTOP', 'GDK_BACKEND'],
                       env=self.env, check=True, timeout=10)
        return dict(self.env)

    def __exit__(self, *exception):
        if self.bus is not None:
            # Stop activated services before removing their runtime directory/FUSE mount.
            for sig in (signal.SIGTERM, signal.SIGKILL):
                try:
                    os.killpg(self.bus.pid, sig)
                except ProcessLookupError:
                    break
                if sig == signal.SIGTERM:
                    time.sleep(.2)
            self.bus.wait(timeout=5)
            self.bus.stdout.close()
        mount = self.directory / 'runtime/doc'
        if os.path.ismount(mount):
            subprocess.run(['fusermount3', '-u', str(mount)], check=True, timeout=5)
        self.temporary.cleanup()
