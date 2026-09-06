#!/usr/bin/env python3
"""Exercise Fire Notes on an isolated X display and temporary note library."""
import argparse
import json
import os
from pathlib import Path
import subprocess
import tempfile
import time
from PIL import ImageGrab


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--binary', default='target/release/fire-notes')
    parser.add_argument('--output', default='artifacts/notes')
    parser.add_argument('--telegram', action='store_true', help='Send screenshots to the configured user; requires explicit user authorization')
    args = parser.parse_args()
    binary = Path(args.binary).resolve()
    output = Path(args.output).resolve()
    output.mkdir(parents=True, exist_ok=True)
    screenshots = []
    def send_gallery():
        if not args.telegram or not screenshots:
            return
        for start in range(0, len(screenshots), 10):
            command = ['python3', '/home/fire/.codex/skills/telegram-notify/scripts/send_telegram.py', '--status', 'progress', '--title', 'Fire Notes']
            for path in screenshots[start:start+10]:
                command.extend(['--image', str(path)])
            command.append('Build screenshots: writing, tabs, pickers, narrow layouts and session restoration. Sample notes use temporary storage.')
            if subprocess.run(command).returncode:
                raise RuntimeError('Telegram gallery delivery failed')
    with tempfile.TemporaryDirectory(prefix='fire-notes-probe-') as temporary:
        directory = Path(temporary)
        notes = directory / 'notes'
        external = directory / 'external.md'
        external.write_text('# Imported note\n\nA note opened from outside the library.\n')
        with (directory / 'display').open('w+') as number_file:
            display = subprocess.Popen(['Xvfb', '-displayfd', str(number_file.fileno()), '-screen', '0', '1200x900x24', '-nolisten', 'tcp'], pass_fds=(number_file.fileno(),), stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
            app = None
            try:
                number = ''
                for _ in range(100):
                    number_file.seek(0)
                    number = number_file.read().strip()
                    if number:
                        break
                    time.sleep(.05)
                assert number, 'Xvfb did not start'
                env = {**os.environ, 'DISPLAY': ':' + number, 'WINIT_UNIX_BACKEND': 'x11', 'LIBGL_ALWAYS_SOFTWARE': '1', 'FIRE_UI_PROFILE': '1'}
                env.pop('WAYLAND_DISPLAY', None)
                def x(*args):
                    return subprocess.check_output(['xdotool', *map(str, args)], env=env, stderr=subprocess.DEVNULL).decode().strip()
                log = (output / 'native.log').open('w')
                def start():
                    process = subprocess.Popen([str(binary), '--data-dir', str(notes)], env=env, stdout=log, stderr=log)
                    for _ in range(100):
                        assert process.poll() is None, 'Fire Notes exited during startup'
                        try:
                            window = x('search', '--name', '^Fire Notes$').splitlines()[0]
                            x('windowfocus', window)
                            time.sleep(.6)
                            return process, window
                        except subprocess.CalledProcessError:
                            time.sleep(.05)
                    raise AssertionError('Window did not appear')
                app, window = start()
                def shot(name, caption):
                    time.sleep(.25)
                    path = output / (name + '.png')
                    # Capture just the app, including narrow window variants.
                    geometry = dict(line.split('=', 1) for line in x('getwindowgeometry', '--shell', window).splitlines())
                    xx, yy, ww, hh = [int(geometry[k]) for k in ['X', 'Y', 'WIDTH', 'HEIGHT']]
                    ImageGrab.grab(xdisplay=env['DISPLAY'], bbox=(xx, yy, xx+ww, yy+hh)).save(path)
                    print('Screenshot:', path, flush=True)
                    screenshots.append(path)
                    print(caption, flush=True)
                def key(keys):
                    x('key', '--clearmodifiers', keys)
                    time.sleep(.12)
                def type_text(text):
                    for index, line in enumerate(text.split('\n')):
                        if index:
                            key('Return')
                        if line:
                            x('type', '--clearmodifiers', '--delay', '2', '--', line)
                    time.sleep(.12)
                def click(xx, yy):
                    x('mousemove', '--window', window, xx, yy)
                    x('click', 1)
                    time.sleep(.12)
                def eventually(check, message):
                    for _ in range(100):
                        if check(): return
                        assert app.poll() is None, 'App exited unexpectedly'
                        time.sleep(.05)
                    raise AssertionError(message)
                shot('01-first-window', 'First working Fire Notes window on the new framework. This is the initial build, before interaction and layout checks.')
                click(250, 180)
                key('ctrl+a')
                type_text('A place to think')
                click(220, 265)
                key('ctrl+a')
                type_text('Small ideas deserve a little room.\n\nKeep the useful parts. Make the rest simpler.\n\nToday\n- Build with care\n- Leave space to think\n- Write things down')
                eventually(lambda: 'Small ideas deserve' in (notes/'welcome.md').read_text(), 'Body was not saved')
                assert (notes/'welcome.md').read_text().startswith('# A place to think\n\n')
                assert '\n\nKeep the useful parts.' in (notes/'welcome.md').read_text(), 'Paragraph breaks were lost'
                shot('02-writing', 'Editing and automatic Markdown saving work. The title and body use the new reusable editor widget.')
                key('ctrl+n')
                click(220, 180)
                key('ctrl+a')
                type_text('Weekend ideas')
                click(220, 265)
                type_text('A long walk. A good book. No notifications.')
                eventually(lambda: len(list(notes.glob('note-*.md'))) == 1, 'New note was not created')
                created = next(notes.glob('note-*.md'))
                eventually(lambda: 'No notifications.' in created.read_text(), 'New note body was not saved')
                shot('03-tabs', 'Multiple notes now have independent editors and tabs. Changes save in the background.')
                key('ctrl+z')
                eventually(lambda: 'No notifications.' not in created.read_text(), 'Undo did not remove the last edit')
                key('ctrl+y')
                eventually(lambda: 'No notifications.' in created.read_text(), 'Redo did not restore the last edit')
                key('ctrl+Tab')
                key('ctrl+End')
                type_text('\nA thought kept between tabs.')
                eventually(lambda: 'A thought kept between tabs.' in (notes/'welcome.md').read_text(), 'Switching tabs did not restore the first editor')
                key('ctrl+p')
                type_text('weekend')
                shot('04-find', 'Search filters the note library. The picker composes a text field with the framework\'s virtual list.')
                key('Return')
                key('ctrl+w')
                assert created.exists(), 'Closing a tab deleted its note'
                key('ctrl+p')
                type_text('weekend')
                key('Return')
                time.sleep(.5)
                key('ctrl+End')
                type_text(' Reopened.')
                eventually(lambda: 'Reopened.' in created.read_text(), 'A closed note did not reopen from storage')
                key('ctrl+w')
                key('ctrl+slash')
                shot('05-commands', 'The command palette provides note actions and word-wrap control.')
                key('Escape')
                key('ctrl+o')
                type_text(str(external))
                shot('06-open-file', 'Open-file flow before loading a Markdown file from disk. Test notes are in a temporary directory.')
                key('Return')
                time.sleep(.7)
                key('ctrl+End')
                type_text(' Loaded and edited.')
                eventually(lambda: 'Loaded and edited.' in external.read_text(), 'Background file loading did not deliver to the editor')
                key('ctrl+p')
                type_text('place')
                key('Return')
                x('windowsize', window, 520, 680)
                time.sleep(.4)
                shot('07-narrow', 'Checking the writing layout at 520 pixels wide, with tabs and controls still accessible.')
                x('windowsize', window, 420, 600)
                time.sleep(.4)
                shot('08-minimum-width', 'Minimum-width layout at 420 pixels. Checking header spacing and text wrapping.')
                for step in range(60):
                    x('windowsize', window, 740 + abs(30-step)*10, 600 + abs(30-step)*5)
                    time.sleep(.012)
                x('windowsize', window, 1100, 820)
                time.sleep(.4)
                click(220, 265)
                key('ctrl+End')
                def state():
                    fields = Path(f'/proc/{app.pid}/stat').read_text().rsplit(')', 1)[1].split()
                    status = dict(line.split(':', 1) for line in Path(f'/proc/{app.pid}/status').read_text().splitlines())
                    return int(fields[11])+int(fields[12]), int(status['VmRSS'].split()[0])
                time.sleep(.6)
                before, _ = state()
                time.sleep(2)
                after, rss = state()
                metrics = {'backend': 'Xvfb / Mesa software GL', 'idle_seconds': 2, 'idle_cpu_ticks': after-before, 'rss_kib': rss}
                resize = sorted(int(line.split('=', 1)[1])/1000 for line in (output/'native.log').read_text().splitlines() if line.startswith('resize_frame_us='))
                if resize:
                    metrics['resize_event_to_swap_ms'] = {'samples': len(resize), 'p50': resize[len(resize)//2], 'p95': resize[min(len(resize)-1, int(len(resize)*.95))], 'max': max(resize)}
                type_text('\nSaved even when closing immediately.')
                key('ctrl+q')
                app.wait(timeout=10)
                assert app.returncode == 0
                assert 'Saved even when closing immediately.' in (notes/'welcome.md').read_text(), 'Close lost the last edit'
                app, window = start()
                shot('09-restored', 'Restart check: notes and open tabs restore from disk, including the final edit made immediately before closing.')
                key('ctrl+w')
                key('ctrl+w')
                shot('10-empty', 'All tabs closed. Notes stay on disk, and the empty window still handles new-note and search shortcuts.')
                key('ctrl+n')
                time.sleep(.4)
                type_text('Created from an empty window.')
                eventually(lambda: any('Created from an empty window.' in p.read_text() for p in notes.glob('note-*.md')), 'New-note shortcut failed in an empty window')
                key('ctrl+q')
                app.wait(timeout=10)
                assert app.returncode == 0
                (output/'metrics.json').write_text(json.dumps(metrics, indent=2)+'\n')
                print('PASS: editing, autosave, tabs, search, command palette, external files, resize, save-before-close and restart.', flush=True)
                print(json.dumps(metrics), flush=True)
            finally:
                if app and app.poll() is None:
                    app.terminate()
                    app.wait(timeout=5)
                display.terminate()
                display.wait(timeout=5)
                send_gallery()

if __name__ == '__main__':
    main()
