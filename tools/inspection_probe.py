#!/usr/bin/env python3
"""Verify semantic automation against a real X11 Fire Notes window and temporary data."""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess
from private_session import PrivateSession
import time
from PIL import ImageGrab


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary', default='target/release/fire-notes')
    parser.add_argument('--output', default='artifacts/inspection')
    args = parser.parse_args()
    binary = str(Path(args.binary).resolve())
    output = Path(args.output).resolve()
    output.mkdir(parents=True, exist_ok=True)
    inspector = Path(__file__).resolve().parents[2] / 'fire-ui/tools/fire_ui_inspect.py'
    with PrivateSession() as session:
        private = session.directory
        socket = private / 'ui.sock'
        with (private / 'display').open('w+') as number_file, (output / 'native.log').open('w') as log:
            display = subprocess.Popen(['Xvfb', '-displayfd', str(number_file.fileno()), '-screen', '0', '1200x900x24', '-nolisten', 'tcp'], pass_fds=(number_file.fileno(),), stdout=log, stderr=log)
            app = None
            try:
                for _ in range(100):
                    number_file.seek(0)
                    number = number_file.read().strip()
                    if number: break
                    time.sleep(.05)
                assert number
                env = session.activate(':' + number)
                env['FIRE_UI_PROFILE'] = '1'
                env['FIRE_UI_INSPECT'] = str(socket)
                app = subprocess.Popen([binary, '--data-dir', str(private/'notes')], env=env, stdout=log, stderr=log)
                for _ in range(100):
                    assert app.poll() is None
                    if socket.exists(): break
                    time.sleep(.05)
                def request(**payload):
                    result = subprocess.run(['python3', str(inspector), str(socket), json.dumps(payload)], capture_output=True, text=True, timeout=6)
                    return json.loads(result.stdout)
                def node(snapshot):
                    return next(n for n in snapshot['nodes'] if n['key'] == 'note-body')
                x = lambda *a: subprocess.check_output(['xdotool', *map(str, a)], env=env, text=True).strip()
                window = x('search', '--name', '^Fire Notes$').splitlines()[0]
                x('windowfocus', window)
                time.sleep(.1)
                snapshot = request()
                assert snapshot['protocol'] == 1
                snapshot = request(label='New note', action='activate')
                assert 'set_value' in node(snapshot)['actions']
                snapshot = request(key='note-body', action='focus')
                assert node(snapshot)['focused']
                original = 'Fire Notes\nEnglish שלום 123 العربية\nCafé e\u0301 👩‍💻'
                snapshot = request(key='note-body', action='set_value', value=original)
                assert node(snapshot)['value'] == original
                snapshot = request(key='note-body', action='set_selection', anchor=10, caret=0)
                assert node(snapshot)['text']['anchor'] == 10 and node(snapshot)['text']['caret'] == 0
                snapshot = request(key='note-body', action='replace_selected_text', value='Agent tested')
                assert node(snapshot)['value'].startswith('Agent tested\n')
                assert 'error' in request(key='note-body', action='set_selection', anchor=0, caret=999999)
                assert 'error' in request(id=999999, action='focus')
                assert 'error' in request(key='note-body', revision=0, action='set_value', value='stale')
                x('key', 'ctrl+z')
                time.sleep(.2)
                assert node(request())['value'] == original
                # Leave mixed-direction selection and animated fire visible for visual review.
                start = len('Fire Notes\nEnglish '.encode())
                request(key='note-body', action='set_selection', anchor=start, caret=start+len('שלום'.encode()))
                time.sleep(.3)
                snapshot = request()
                (output/'tree.json').write_text(json.dumps(snapshot, ensure_ascii=False, indent=2))
                ImageGrab.grab(xdisplay=env['DISPLAY']).save(output/'bidi-selection.png')
                # Allow fire to stop and confirm the inspection server has no polling CPU cost.
                request(key='note-body', action='set_selection', anchor=0, caret=0)
                request(label='New note', action='focus')
                time.sleep(1.2)
                def ticks():
                    fields = Path(f'/proc/{app.pid}/stat').read_text().rsplit(')', 1)[1].split()
                    return int(fields[11])+int(fields[12])
                before=ticks(); time.sleep(2); idle=ticks()-before
                request(label='Close window', action='activate')
                app.wait(timeout=8)
                assert not socket.exists(), 'inspection socket was not cleaned up'
                results = {'binary_sha256': hashlib.sha256(Path(binary).read_bytes()).hexdigest(), 'semantic_editing': 'passed', 'native_keyboard_undo': 'passed', 'invalid_and_stale_requests': 'rejected',
                           'idle_cpu_ticks_over_2_seconds': idle, 'socket_cleanup': 'passed', 'temporary_data': True}
                (output/'results.json').write_text(json.dumps(results, indent=2))
                print(json.dumps(results, indent=2))
            finally:
                if app and app.poll() is None: app.terminate(); app.wait(timeout=8)
                display.terminate(); display.wait(timeout=5)


if __name__ == '__main__':
    main()
