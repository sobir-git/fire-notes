#!/usr/bin/env python3
"""Native probe for the inline checklist feature on an isolated X display.

Types Markdown task lists the way the feature intends — the first marker by
hand, then only the text after each Return, because Enter on a task line
auto-creates the next item and Enter on an empty item exits the list — and
verifies the rendered checkboxes replace the raw syntax, toggles them by
pointer, checks one-step undo/redo, Ctrl+Enter, and captures the
source-reveal state. Runs against a temporary note library; the installed
app and the user's notes are untouched.
"""
from pathlib import Path
import argparse
import subprocess
import time
from PIL import ImageGrab

from private_session import PrivateSession


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--binary', type=Path, default=Path('target/release/fire-notes'))
    args = parser.parse_args()
    output = Path('artifacts/checklist')
    output.mkdir(parents=True, exist_ok=True)
    binary = args.binary.resolve()
    with PrivateSession() as session:
        directory = session.directory
        notes = directory / 'notes'
        with (directory / 'display').open('w+') as number_file:
            display = subprocess.Popen(
                ['Xvfb', '-displayfd', str(number_file.fileno()), '-screen', '0',
                 '900x700x24', '-nolisten', 'tcp'],
                pass_fds=(number_file.fileno(),),
                stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
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
                env = session.activate(':' + number)

                def x(*args):
                    return subprocess.check_output(
                        ['xdotool', *map(str, args)], env=env,
                        stderr=subprocess.DEVNULL).decode().strip()

                log = (output / 'native.log').open('w')
                app = subprocess.Popen(
                    [str(binary), '--data-dir', str(notes)], env=env,
                    stdout=log, stderr=log)
                window = None
                for _ in range(100):
                    assert app.poll() is None, 'Fire Notes exited during startup'
                    try:
                        window = x('search', '--name', '^Fire Notes$').splitlines()[0]
                        x('windowfocus', window)
                        time.sleep(.6)
                        break
                    except subprocess.CalledProcessError:
                        time.sleep(.05)
                assert window, 'Window did not appear'

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

                def move(xx, yy):
                    x('mousemove', '--window', window, xx, yy)

                def click(xx, yy):
                    move(xx, yy)
                    x('click', 1)
                    time.sleep(.25)

                def clear_reveal(yy):
                    # A click past the marker text parks the caret outside
                    # any hidden syntax; while the raw source is revealed the
                    # box target steps aside, so every box attempt starts
                    # from a clean, unrevealed row.
                    move(260, yy)
                    x('click', 1)
                    time.sleep(.2)

                def body_text():
                    files = list(notes.glob('note-*.md'))
                    return files[0].read_text() if len(files) == 1 else ''

                def shot(name):
                    time.sleep(.25)
                    geometry = dict(
                        line.split('=', 1)
                        for line in x('getwindowgeometry', '--shell', window).splitlines())
                    xx, yy = int(geometry['X']), int(geometry['Y'])
                    ww, hh = int(geometry['WIDTH']), int(geometry['HEIGHT'])
                    captured = ImageGrab.grab(
                        xdisplay=env['DISPLAY'],
                        bbox=(xx, yy, xx + ww, yy + hh))
                    captured.save(output / f'{name}.png')
                    print('Screenshot:', output / f'{name}.png', flush=True)
                    return captured

                x('windowsize', window, 900, 600)
                key('ctrl+n')
                # Type the way the feature intends: the first marker by hand,
                # then only the text — each Return on a task line auto-creates
                # the next `- [ ] `, and a second Return on the empty item
                # exits the list for the trailing plain-text line.
                body = '- [ ] buy milk\n- [ ] ship the release\n- [ ] call mom\nplain text'
                type_text('- [ ] buy milk')
                key('Return')
                x('type', '--clearmodifiers', '--delay', '2', '--', 'ship the release')
                key('Return')
                x('type', '--clearmodifiers', '--delay', '2', '--', 'call mom')
                key('Return')
                key('Return')
                x('type', '--clearmodifiers', '--delay', '2', '--', 'plain text')
                time.sleep(.12)
                for _ in range(100):
                    if body in body_text():
                        break
                    assert app.poll() is None, 'App exited unexpectedly'
                    time.sleep(.05)
                assert body in body_text(), 'Typed checklist was not saved'
                time.sleep(1.5)
                shot('01-typed')
                # Scan the left margin of the first rows until a click toggles
                # the first box; the on-disk Markdown is the source of truth.
                first_box = None
                for yy in range(50, 70, 2):
                    for xx in (34, 38, 42, 46, 30):
                        clear_reveal(yy)
                        move(xx, yy)
                        x('click', 1)
                        time.sleep(.25)
                        if '- [x] buy milk' in body_text():
                            first_box = (xx, yy)
                            break
                        if body_text() != body:
                            raise AssertionError(f'Click at {xx},{yy} corrupted the note')
                    if first_box:
                        break
                assert first_box, 'No click in the checkbox area toggled the first item'
                print(f'Checkbox toggle on at {first_box}.', flush=True)
                time.sleep(.05)
                shot('02-checked')
                time.sleep(.3)
                # Clicking the same box again toggles it back off.
                move(*first_box)
                x('click', 1)
                time.sleep(.3)
                assert '- [ ] buy milk' in body_text(), 'Toggle off failed'
                print('Toggle off verified on disk.', flush=True)
                # The second row starts unchecked; clicking its box checks it.
                # The row pitch is the editor line height, so scan a pitch band.
                second_box = None
                for pitch in range(14, 40, 2):
                    yy = first_box[1] + pitch
                    clear_reveal(yy)
                    move(first_box[0], yy)
                    x('click', 1)
                    time.sleep(.25)
                    if '- [x] ship the' in body_text():
                        second_box = (first_box[0], yy)
                        break
                assert second_box, 'Clicking the second box did not check it'
                print('Second item checked from its box.', flush=True)
                shot('03-checked-second')
                # One undo reverts the check; one redo reapplies it.
                key('ctrl+z')
                deadline = time.time() + 5
                while '- [x] ship the' in body_text():
                    assert app.poll() is None, 'App exited unexpectedly'
                    assert time.time() < deadline, 'Undo did not revert the check'
                    time.sleep(.05)
                print('Undo reverted the check in one step.', flush=True)
                key('ctrl+y')
                deadline = time.time() + 5
                while '- [x] ship the' not in body_text():
                    assert app.poll() is None, 'App exited unexpectedly'
                    assert time.time() < deadline, 'Redo did not reapply the check'
                    time.sleep(.05)
                print('Redo reapplied the check.', flush=True)
                # Ctrl+Enter toggles the task under the caret: click into the
                # third item's text, then press the shortcut.
                third_y = second_box[1] + (second_box[1] - first_box[1])
                move(first_box[0] + 60, third_y)
                x('click', 1)
                time.sleep(.25)
                key('ctrl+Return')
                deadline = time.time() + 5
                while '- [x] call mom' not in body_text():
                    assert app.poll() is None, 'App exited unexpectedly'
                    assert time.time() < deadline, 'Ctrl+Enter did not toggle the item'
                    time.sleep(.05)
                print('Ctrl+Enter toggled the item under the caret.', flush=True)
                # Park the caret inside the first marker: the raw `- [ ]`
                # syntax is revealed and the box steps aside.
                move(75, first_box[1])
                x('click', 1)
                time.sleep(.25)
                for _ in range(5):
                    key('Left')
                shot('04-source-revealed')
                # Drag-select the whole plain-text line and type over it: the
                # stored Markdown proves pointer selection stayed live.
                plain_y = first_box[1] + 3 * (second_box[1] - first_box[1])
                clear_reveal(plain_y)
                x('mousemove', '--window', window, 14, plain_y)
                x('mousedown', 1)
                time.sleep(.15)
                x('mousemove', '--window', window, 120, plain_y)
                time.sleep(.1)
                x('mousemove', '--window', window, 200, plain_y)
                time.sleep(.15)
                x('mouseup', 1)
                time.sleep(.3)
                x('type', '--clearmodifiers', '--delay', '2', '--', 'plain stuff')
                time.sleep(.3)
                # Plain text after the list is untouched by rendering, and the
                # accumulated toggles plus the drag replacement left exactly
                # the expected Markdown.
                final = '- [ ] buy milk\n- [x] ship the release\n- [x] call mom\nplain stuff'
                assert final in body_text(), \
                    f'Rendering, toggles or drag selection altered the stored Markdown: {body_text()!r}'
                # Regression: inspect selection pixels while the button is
                # still down, before release can hide a missing repaint.
                def reset_task():
                    key('ctrl+a')
                    type_text('- [ ] buy milk')
                    key('ctrl+End')
                    time.sleep(.6)

                reset_task()
                row_y = first_box[1] + 4
                move(155, row_y)
                x('mousedown', 1)
                time.sleep(.15)
                for xx in (130, 110, 90, 74):
                    move(xx, row_y)
                    time.sleep(.1)
                held = shot('05-selection-held')
                pixels = held.crop((75, 50, 150, 69)).getdata()
                orange = sum(r > 35 and r > g * 1.5 and r > b * 1.5 for r, g, b in pixels)
                assert orange > 200, f'Selection did not paint while held: {orange} orange pixels'
                x('mouseup', 1)
                shot('06-selection-released')
                type_text('selected')
                time.sleep(.6)
                assert body_text() == '- [ ] selected', body_text()

                reset_task()
                key('ctrl+shift+Left')
                key('Return')
                time.sleep(.6)
                assert body_text() == '- [ ] buy \n- [ ] ', body_text()
                key('ctrl+z')
                time.sleep(.6)
                assert body_text() == '- [ ] buy milk', body_text()

                reset_task()
                move(*first_box)
                x('mousedown', 1)
                time.sleep(.15)
                for xx in (60, 90, 120, 155):
                    move(xx, row_y)
                    time.sleep(.1)
                shot('07-selection-from-checkbox')
                x('mouseup', 1)
                type_text('dragged')
                time.sleep(.6)
                dragged = body_text()
                assert 'buy milk' not in dragged and 'dragged' in dragged, dragged
                assert '[x]' not in dragged, 'Dragging must not toggle the checkbox'
                reset_task()
                key('Return')
                key('Return')
                type_text('caret here')
                time.sleep(2.)
                move(*first_box)
                x('click', 1)
                toggled = shot('08-toggle-with-distant-caret')
                assert '- [x] buy milk' in body_text(), 'Top checkbox did not toggle'
                # The ordinary orange caret is at x=112; inspect the text
                # immediately before it, where an insertion burst would burn.
                pixels = toggled.crop((60, 72, 110, 103)).getdata()
                fire = sum(r > 60 and r > g * 1.5 and r > b * 1.5 for r, g, b in pixels)
                assert fire < 35, f'Checkbox activation emitted fire at the distant caret: {fire} pixels'
                print('PASS: selection paints before release; Enter replaces selection '
                      'and undoes in one step; checkbox drags select text; '
                      'checkbox activation does not emit fire at the caret.', flush=True)
                key('ctrl+q')
                app.wait(timeout=10)
                assert app.returncode == 0
                print('PASS: Enter continuation, checklist parsing, checkbox '
                      'rendering, click toggling, one-step undo/redo, '
                      'Ctrl+Enter, source reveal, drag selection and verbatim '
                      'Markdown on native Xvfb with temporary data.',
                      flush=True)
            except Exception:
                ImageGrab.grab(xdisplay=env['DISPLAY']).save(output / 'failure.png')
                raise
            finally:
                if app and app.poll() is None:
                    app.terminate()
                    app.wait(timeout=5)
                display.terminate()
                display.wait(timeout=5)


if __name__ == '__main__':
    main()
