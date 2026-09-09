#!/usr/bin/env python3
"""Exercise tab context actions on an isolated X display and temporary note library."""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess
from private_session import PrivateSession
import time
from PIL import ImageGrab


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--binary', default='target/release/fire-notes')
    parser.add_argument('--output', default='artifacts/tab-menu')
    args = parser.parse_args()
    binary = Path(args.binary).resolve()
    output = Path(args.output).resolve()
    output.mkdir(parents=True, exist_ok=True)
    fingerprint=hashlib.sha256(binary.read_bytes()).hexdigest()
    screenshots = []
    with PrivateSession() as session:
        directory = session.directory
        notes = directory / 'notes'
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
                env = session.activate(':' + number)
                env['FIRE_UI_PROFILE'] = '1'
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
                x('windowsize', window, 740, 480)
                first=notes/'note-1.md'
                type_text('Fire Notes\n\nRight-click a tab to manage your notes.')
                key('ctrl+r');key('ctrl+a');type_text('First note');key('Return')
                key('ctrl+n');type_text('A second note.');key('ctrl+r');key('ctrl+a');type_text('Second note');key('Return')
                second=next(p for p in notes.glob('note-*.md') if p!=first)
                key('ctrl+1');key('ctrl+End')
                for name,yy in [('00-hover-padding',5),('00-hover-text',20)]:
                    x('mousemove','--window',window,180,yy)
                    shot(name, 'Tab hover covers both its label and padding.')
                    from PIL import Image
                    assert Image.open(screenshots[-1]).getpixel((150,5))[:3]==(59,38,29), 'Tab hover disappeared over text'
                x('mousemove','--window',window,400,300);time.sleep(.2)
                def saved():return json.loads((notes/'session.json').read_text())
                def menu(xx):
                    x('mousemove','--window',window,xx,20);x('click',3);time.sleep(.2)
                menu(180)
                shot('01-actions', 'Right-click actions on an inactive tab.')
                assert saved()['active']==str(first), 'Opening menu switched active tab'
                click(225,105)
                eventually(lambda:saved()['views'][str(second)]['wrap'],'Wrap targeted the wrong tab')
                assert saved()['active']==str(first)
                menu(180)
                shot('02-checked-wrap', 'Word wrap reflects the clicked tab, with keyboard hints.')
                key('Escape');type_text(' Focus restored.')
                eventually(lambda:first.read_text().endswith(' Focus restored.'),'Dismiss did not restore editor focus')
                assert second.read_text()=='A second note.'
                menu(180);key('Down');key('Return');key('ctrl+a');type_text('Ideas');key('Return')
                shot('03-renamed', 'Rename activates and edits the clicked tab.')
                eventually(lambda:saved()['titles'][str(second)]=='Ideas','Context rename failed')
                assert saved()['active']==str(second)
                menu(50);click(100,73)
                assert saved()['active']==str(second),'Save switched tabs'
                menu(50);key('Home');key('Down');key('Down');key('Down');key('Return')
                eventually(lambda:saved()['tabs']==[str(second)],'Close targeted the wrong tab')
                assert first.exists(),'Close deleted a note'
                menu(50)
                shot('04-last-tab', 'Close is disabled for the last tab; note files are retained.')
                click(100,137);time.sleep(.2)
                assert saved()['tabs']==[str(second)]
                key('Escape')
                menu(50);click(400,300);key('ctrl+End');type_text(' Outside dismissed.')
                eventually(lambda:second.read_text().endswith(' Outside dismissed.'),'Outside dismissal lost editor focus')
                # Trash the last open tab, restore it, and exercise retention through the CLI.
                key('ctrl+End');type_text(' Last input before Trash.')
                expected=second.read_text() if second.read_text().endswith('Last input before Trash.') else 'A second note. Outside dismissed. Last input before Trash.'
                key('ctrl+s');key('ctrl+s')
                menu(50);click(100,169)
                eventually(lambda:not second.exists(),'Move to Trash left the note in the library')
                entries=list((notes/'trash').glob('*/entry.json'))
                assert len(entries)==1
                entry=json.loads(entries[0].read_text())
                assert entry['title']=='Ideas' and entry['view']['wrap']
                assert (entries[0].parent/'note.md').read_text()==expected,'Trash lost recent input'
                assert saved()['tabs']==[], 'Trashing the last tab did not close it'
                key('ctrl+shift+t')
                shot('05-trash', 'Trash shows recovery time; Enter or click restores the selected note.')
                type_text('Ideas');key('Return')
                eventually(lambda:second.exists() and saved()['active']==str(second),'Restore failed')
                assert second.read_text()==expected
                assert saved()['titles'][str(second)]=='Ideas' and saved()['views'][str(second)]['wrap']
                shot('06-restored-note', 'Restored note retains its title, text and wrapping.')
                menu(50);click(100,169);key('ctrl+q')
                app.wait(timeout=10);assert app.returncode==0
                entries=list((notes/'trash').glob('*/entry.json'));assert len(entries)==1
                entry=json.loads(entries[0].read_text())
                entry['deleted_at']=int(time.time())-30*86400+3600
                entries[0].write_text(json.dumps(entry))
                subprocess.run([str(binary),'--data-dir',str(notes),'--purge-trash'],check=True,env=env)
                assert (entries[0].parent/'note.md').exists(),'Cleanup deleted an unexpired note'
                entry['deleted_at']=int(time.time())-30*86400
                entries[0].write_text(json.dumps(entry))
                subprocess.run([str(binary),'--data-dir',str(notes),'--purge-trash'],check=True,env=env)
                assert not (entries[0].parent/'note.md').exists(),'Expired note was not deleted'
                assert first.exists(),'Trash cleanup touched an active-library note'
                app,window=start();key('ctrl+shift+t')
                shot('07-empty-trash', 'Expired notes are removed automatically; active notes remain untouched.')
                key('Escape')
                key('ctrl+q');app.wait(timeout=10);assert app.returncode==0
                (output/'result.json').write_text(json.dumps({'binary_sha256':fingerprint,'passed':['inactive tab target','wrap','focus restoration','rename','save','close preserves files','last-tab disabled','outside dismiss','hover over text and padding','trash latest input','restore title and view','last-tab trash','close waits for trash','30-day cleanup']},indent=2)+'\n')
                print('PASS: native tab context actions',flush=True)
            finally:
                if app and app.poll() is None:
                    app.terminate()
                    app.wait(timeout=5)
                display.terminate()
                display.wait(timeout=5)

if __name__ == '__main__':
    main()
