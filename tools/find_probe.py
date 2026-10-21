#!/usr/bin/env python3
"""Exercise find in a note with real X11 keys and clicks on a private display."""
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
    parser.add_argument('--output', default='artifacts/find')
    args = parser.parse_args()
    binary = Path(args.binary).resolve()
    output = Path(args.output).resolve()
    output.mkdir(parents=True, exist_ok=True)
    inspector = Path(__file__).resolve().parents[2] / 'fire-ui/tools/fire_ui_inspect.py'
    fingerprint = hashlib.sha256(binary.read_bytes()).hexdigest()
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
                    if number:
                        break
                    time.sleep(.05)
                assert number, 'Xvfb did not start'
                env = session.activate(':' + number)
                env['FIRE_UI_INSPECT'] = str(socket)
                notes = private / 'notes'
                app = subprocess.Popen([str(binary), '--data-dir', str(notes)], env=env, stdout=log, stderr=log)
                for _ in range(100):
                    assert app.poll() is None, 'Fire Notes exited during startup'
                    if socket.exists():
                        break
                    time.sleep(.05)
                def request(**payload):
                    result = subprocess.run(['python3', str(inspector), str(socket), json.dumps(payload)], capture_output=True, text=True, timeout=6)
                    return json.loads(result.stdout)
                def x(*a):
                    return subprocess.check_output(['xdotool', *map(str, a)], env=env, text=True).strip()
                window = x('search', '--name', '^Fire Notes$').splitlines()[0]
                x('windowsize', window, 740, 480)
                x('windowfocus', window)
                time.sleep(.3)
                def node(snapshot, key):
                    return next((n for n in snapshot['nodes'] if n.get('key') == key), None)
                def status(snapshot):
                    bar = node(snapshot, 'find-bar')
                    return next((n['label'] for n in snapshot['nodes'] if n.get('parent') == bar['id'] and n['role'] == 'Text'), None)
                def selection(snapshot):
                    text = node(snapshot, 'note-body')['text']
                    return text['anchor'], text['caret']
                def eventually(check, message):
                    for _ in range(100):
                        snapshot = request()
                        if check(snapshot):
                            return snapshot
                        assert app.poll() is None, 'Fire Notes exited unexpectedly'
                        time.sleep(.05)
                    raise AssertionError(f'{message}: {json.dumps(snapshot)[:2000]}')
                def key(keys):
                    x('key', '--clearmodifiers', keys)
                    time.sleep(.12)
                def type_text(text):
                    x('type', '--clearmodifiers', '--delay', '2', '--', text)
                    time.sleep(.12)
                def click(label):
                    bounds = next(n for n in request()['nodes'] if n['label'] == label)['bounds']
                    x('mousemove', '--window', window, int(bounds['x'] + bounds['width'] / 2), int(bounds['y'] + bounds['height'] / 2))
                    x('click', 1)
                    x('mousemove', '--window', window, 400, 200)
                    time.sleep(.12)
                def shot(name):
                    time.sleep(.3)
                    geometry = dict(line.split('=', 1) for line in x('getwindowgeometry', '--shell', window).splitlines())
                    xx, yy, ww, hh = [int(geometry[k]) for k in ['X', 'Y', 'WIDTH', 'HEIGHT']]
                    path = output / f'{name}.png'
                    ImageGrab.grab(xdisplay=env['DISPLAY'], bbox=(xx, yy, xx + ww, yy + hh)).save(path)
                    print('Screenshot:', path, flush=True)
                lines = [f'Line {i}: kindling, smoke and sparks.' for i in range(1, 41)]
                lines[1] = 'Line 2: an Ember wakes the hearth.'
                lines[11] = 'Line 12: every ember counts.'
                lines[37] = 'Line 38: EMBER glow, far below.'
                text = '\n'.join(lines)
                request(key='note-body', action='focus')
                request(key='note-body', action='set_value', value=text)
                request(key='note-body', action='set_selection', anchor=0, caret=0)
                found = []
                at = 0
                while (at := text.lower().find('ember', at)) >= 0:
                    found.append((at, at + 5))
                    at += 5
                assert len(found) == 3
                key('ctrl+f')
                snapshot = eventually(lambda s: node(s, 'find-query') and node(s, 'find-query')['focused'], 'Ctrl F did not focus the query')
                type_text('ember')
                snapshot = eventually(lambda s: status(s) == '1 of 3', 'Typing did not find the first occurrence')
                assert selection(snapshot) == found[0]
                shot('01-first-of-three')
                key('Return')
                key('Return')
                snapshot = eventually(lambda s: status(s) == '3 of 3', 'Enter did not step forward')
                assert selection(snapshot) == found[2]
                shot('02-third-revealed')
                key('shift+Return')
                snapshot = eventually(lambda s: status(s) == '2 of 3', 'Shift Enter did not step back')
                assert selection(snapshot) == found[1]
                click('Next match')
                eventually(lambda s: status(s) == '3 of 3', 'Next button did not step forward')
                click('Previous match')
                eventually(lambda s: status(s) == '2 of 3', 'Previous button did not step back')
                type_text('x')
                snapshot = eventually(lambda s: status(s) == 'No results', 'A missing query reported matches')
                assert selection(snapshot) == found[1], 'A missing query moved the selection'
                shot('03-no-results')
                key('BackSpace')
                eventually(lambda s: status(s) == '2 of 3', 'Refining kept no current occurrence')
                key('Escape')
                snapshot = eventually(lambda s: node(s, 'find-bar') is None and node(s, 'note-body')['focused'], 'Escape did not return to the note')
                assert app.poll() is None, 'Escape closed the window'
                assert selection(snapshot) == found[1]
                shot('04-selected-on-fire')
                type_text('EMBERS')
                expected = text[:found[1][0]] + 'EMBERS' + text[found[1][1]:]
                eventually(lambda s: node(s, 'note-body')['value'] == expected, 'Typing after find did not replace the match')
                key('ctrl+f')
                eventually(lambda s: node(s, 'find-query') and node(s, 'find-query')['focused'], 'Reopening did not focus the query')
                type_text('glow')
                eventually(lambda s: status(s) == '1 of 1', 'Reopened query did not replace the old one')
                x('windowsize', window, 420, 360)
                shot('05-narrow')
                click('Close find')
                eventually(lambda s: node(s, 'find-bar') is None and node(s, 'note-body')['focused'], 'Close button did not return to the note')
                key('ctrl+q')
                app.wait(timeout=10)
                assert app.returncode == 0
                (output / 'result.json').write_text(json.dumps({'binary_sha256': fingerprint, 'passed': ['ctrl f focuses query', 'incremental first occurrence', 'enter and shift enter step', 'step buttons', 'no results keeps selection', 'escape returns focus without closing', 'typing replaces the match', 'reopen replaces query', 'narrow layout', 'close button']}, indent=2) + '\n')
                print('PASS: native find in note', flush=True)
            finally:
                if app and app.poll() is None:
                    app.terminate()
                    app.wait(timeout=5)
                display.terminate()
                display.wait(timeout=5)


if __name__ == '__main__':
    main()
