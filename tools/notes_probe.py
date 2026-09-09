#!/usr/bin/env python3
"""Exercise Fire Notes on an isolated X display and temporary note library."""
import argparse
import ctypes
import hashlib
import json
from pathlib import Path
import subprocess
from private_session import PrivateSession
import time
from PIL import ImageGrab, ImageChops, Image


def notify_position(display_name,window,x,y,width,height):
    """Supply the ICCCM notification a window manager normally sends after moving.

    Xvfb alone has no WM. winit deliberately ignores nonsynthetic move events,
    whose coordinates may be relative to a reparenting frame.
    """
    class Configure(ctypes.Structure):
        _fields_=[('type',ctypes.c_int),('serial',ctypes.c_ulong),('send_event',ctypes.c_int),('display',ctypes.c_void_p),('event',ctypes.c_ulong),('window',ctypes.c_ulong),('x',ctypes.c_int),('y',ctypes.c_int),('width',ctypes.c_int),('height',ctypes.c_int),('border_width',ctypes.c_int),('above',ctypes.c_ulong),('override_redirect',ctypes.c_int)]
    class Event(ctypes.Union):
        _fields_=[('configure',Configure),('pad',ctypes.c_long*24)]
    lib=ctypes.CDLL('libX11.so.6')
    lib.XOpenDisplay.argtypes=[ctypes.c_char_p];lib.XOpenDisplay.restype=ctypes.c_void_p
    lib.XSendEvent.argtypes=[ctypes.c_void_p,ctypes.c_ulong,ctypes.c_int,ctypes.c_long,ctypes.POINTER(Event)]
    lib.XFlush.argtypes=[ctypes.c_void_p];lib.XCloseDisplay.argtypes=[ctypes.c_void_p]
    connection=lib.XOpenDisplay(display_name.encode());assert connection
    try:
        event=Event();event.configure=Configure(22,0,1,connection,int(window),int(window),x,y,width,height,0,0,0)
        assert lib.XSendEvent(connection,int(window),0,1<<17,ctypes.byref(event))
        lib.XFlush(connection)
    finally:lib.XCloseDisplay(connection)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--binary', default='target/release/fire-notes')
    parser.add_argument('--output', default='artifacts/notes')
    parser.add_argument('--telegram', action='store_true', help='Send screenshots to the configured user; requires explicit user authorization')
    args = parser.parse_args()
    binary = Path(args.binary).resolve()
    output = Path(args.output).resolve()
    output.mkdir(parents=True, exist_ok=True)
    fingerprint=hashlib.sha256(binary.read_bytes()).hexdigest()
    screenshots = []
    def send_gallery():
        if not args.telegram or not screenshots:
            return
        start=0
        while start<len(screenshots):
            count=9 if len(screenshots)-start==11 else min(10,len(screenshots)-start)
            command = ['python3', '/home/fire/.codex/skills/telegram-notify/scripts/send_telegram.py', '--status', 'progress', '--title', 'Fire Notes']
            for path in screenshots[start:start+count]:
                command.extend(['--image', str(path)])
            start+=count
            command.append('Build screenshots: writing, tabs, pickers, narrow layouts and session restoration. Sample notes use temporary storage.')
            if subprocess.run(command).returncode:
                raise RuntimeError('Telegram gallery delivery failed')
    with PrivateSession() as session:
        directory = session.directory
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
                x('windowsize', window, 900, 600)
                shot('01-empty', 'Original compact chrome on the new framework.')
                time.sleep(.5)
                assert not list(notes.glob('*.md')), 'Untouched startup draft was saved'
                key('ctrl+n');time.sleep(.5);key('ctrl+w')
                assert not list(notes.glob('*.md')), 'Empty new tab left a note file'
                key('ctrl+r');key('Return');time.sleep(.4)
                assert not list(notes.glob('*.md')), 'Unchanged default name saved a draft'
                key('ctrl+q');app.wait(timeout=10);assert app.returncode==0
                state=json.loads((notes/'session.json').read_text())
                assert not state['tabs'] and not state['titles'] and not state['views']
                assert not list(notes.glob('*.md')), 'Quit saved an untouched draft'
                assert not list((notes/'trash').glob('*/entry.json')), 'Draft went to Trash'
                app,window=start()
                shot('01a-discarded', 'Empty Untitled drafts leave no files or restored tabs after restart.')
                key('ctrl+n')
                body='Fire Notes\n\nA small, fast place for your thoughts.\nKeep the useful parts. Make the rest simpler.'
                type_text(body)
                eventually(lambda:bool(list(notes.glob('note-*.md'))),'Typing did not save the draft')
                first=next(notes.glob('note-*.md'))
                eventually(lambda: first.read_text()==body, 'Initial typing or raw Markdown saving failed')
                time.sleep(1.8)
                shot('02-writing', 'Text placement and colors use the original app as reference.')
                key('ctrl+a')
                shot('03-fire-selection', 'Selected text glows and emits rising fire particles.')
                shot('04-fire-next-frame', 'A second frame verifies that the fire is animated.')
                assert ImageChops.difference(Image.open(screenshots[-2]),Image.open(screenshots[-1])).getbbox(), 'Fire did not animate'
                key('Right')
                key('ctrl+n'); type_text('A second note.')
                key('ctrl+r');key('ctrl+a');type_text('Ideas')
                shot('05-rename', 'Inline tab rename, matching the original.')
                key('Return')
                eventually(lambda: len(list(notes.glob('note-*.md')))==2,'New tab was not saved')
                created=next(p for p in notes.glob('note-*.md') if p!=first)
                eventually(lambda: created.read_text()=='A second note.','Second editor lost text')
                x('mousemove','--window',window,170,20);x('click',2);time.sleep(.2)
                key('ctrl+a');type_text('Draft');key('Escape')
                x('click',3);time.sleep(.2);key('Down');key('Return');key('ctrl+a');type_text('Ideas')
                click(40,65)  # Clicking the editor commits rename without stealing its focus.
                # Drag the second tab ahead of the first and back.
                x('mousemove','--window',window,170,20);x('mousedown',1);x('mousemove','--window',window,40,20);time.sleep(.2);x('mouseup',1)
                key('ctrl+1');key('ctrl+End');type_text(' Dragged.')
                eventually(lambda:'Dragged.' in created.read_text(),'Tab drag did not reorder')
                for _ in ' Dragged.':key('ctrl+z')
                x('mousemove','--window',window,40,20);x('mousedown',1);x('mousemove','--window',window,170,20);time.sleep(.2);x('mouseup',1)
                key('ctrl+2')
                key('ctrl+z');eventually(lambda:created.read_text()!='A second note.','Undo failed')
                key('ctrl+y');eventually(lambda:created.read_text()=='A second note.','Redo failed')
                key('ctrl+a');key('ctrl+c');key('Right');key('ctrl+v')
                eventually(lambda:created.read_text()=='A second note.A second note.','Clipboard copy/paste failed')
                key('ctrl+z');eventually(lambda:created.read_text()=='A second note.','Paste undo failed')
                key('ctrl+a');key('ctrl+x');eventually(lambda:created.read_text()=='','Cut failed')
                key('ctrl+z');eventually(lambda:created.read_text()=='A second note.','Cut undo failed')
                key('alt+z')
                key('ctrl+1');key('ctrl+Home');key('Right');key('Right')
                key('ctrl+2');key('ctrl+1');type_text('!')
                eventually(lambda:first.read_text().startswith('Fi!re Notes'),'Tab switch lost caret')
                key('ctrl+z')
                key('ctrl+p')
                shot('06-picker', 'Original compact note picker, with open-note indicators.')
                type_text('ideas');key('Return');key('ctrl+w')
                assert created.exists(),'Closing a tab deleted its file'
                key('ctrl+p');type_text('ideas');key('Return');time.sleep(.4)
                key('ctrl+End');type_text(' Reopened.')
                eventually(lambda:'Reopened.' in created.read_text(),'Closed tab failed to reopen')
                key('ctrl+1');key('ctrl+End');key('ctrl+slash')
                shot('07-commands', 'Command palette anchored to the editor caret.')
                type_text('no matching command')
                shot('07a-empty-commands', 'Empty command search uses the same warm popup styling.')
                key('ctrl+a');type_text('wrap');key('Return')
                x('windowsize',window,420,360);time.sleep(.2)
                key('ctrl+slash')
                shot('07b-narrow-commands', 'The palette fits the minimum window size without covering its search or rows.')
                key('Escape');key('ctrl+p');type_text('no matching note')
                shot('07c-empty-notes', 'Note search stays legible at minimum size, including no-results feedback.')
                key('Escape');x('windowsize',window,900,600)
                # Native chooser, tested in its actual separate desktop window.
                key('ctrl+o')
                def choose_file(path):
                    dialog=None
                    for _ in range(100):
                        windows=[]
                        for pattern in ['zenity','kdialog','xdg-desktop-portal-gtk']:
                            try: windows+=x('search','--onlyvisible','--class',pattern).splitlines()
                            except subprocess.CalledProcessError:pass
                        if windows: dialog=windows[-1];break
                        time.sleep(.05)
                    assert dialog,'Native file chooser did not appear'
                    time.sleep(.8)  # GTK maps its top-level before the chooser is ready for focus.
                    x('windowfocus',dialog);time.sleep(.2);key('ctrl+l');key('ctrl+a')
                    subprocess.run(['xclip','-selection','clipboard'],input=str(path).encode(),env=env,stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL,check=True)
                    key('ctrl+v');time.sleep(.3);key('Return');time.sleep(.5)
                    # GTK location entry may require a second Return to accept the file.
                    try:
                        x('getwindowname',dialog);key('Return')
                    except subprocess.CalledProcessError:pass
                    time.sleep(.5);x('windowfocus',window)
                choose_file(external)
                eventually(lambda:json.loads((notes/'session.json').read_text()).get('active')==str(external),'Native chooser did not activate the selected file')
                key('ctrl+End');type_text(' Loaded and edited.')
                eventually(lambda:'Loaded and edited.' in external.read_text(),'Native chooser did not open the selected file')
                assert external.read_text().startswith('# Imported note\n\n'),'Opening stripped the Markdown heading'
                shot('08-imported', 'Native file opening preserves raw Markdown content.')
                exported=directory/'exported.md'
                key('ctrl+shift+s');choose_file(exported)
                eventually(lambda:exported.exists(),'Save As did not create the chosen file')
                assert exported.read_text()==external.read_text(),'Save As changed the file contents'
                key('ctrl+1')
                x('windowsize',window,420,600);time.sleep(.3)
                shot('09-narrow', 'Minimum-width layout with per-tab word wrapping.')
                x('windowsize',window,900,600)
                key('ctrl+n');type_text('\n'.join(f'Line {i:03d}: scroll and selection' for i in range(100)))
                long_note=None
                def find_long():
                    nonlocal long_note
                    long_note=next((p for p in notes.glob('note-*.md') if 'Line 099' in p.read_text()),None)
                    return long_note is not None
                eventually(find_long,'Long note did not save')
                key('ctrl+Home')
                x('mousemove','--window',window,895,530);x('mousedown',1);x('mousemove','--window',window,895,580);x('mouseup',1);time.sleep(.3)
                shot('10-scrollbar', 'Dragging the editor scrollbar keeps the caret in its original place.')
                key('ctrl+1');key('ctrl+4')
                key('ctrl+s');time.sleep(.5)
                # Stress resize while selection is inactive, then measure fully settled idle.
                for step in range(60):
                    x('windowsize',window,740+abs(30-step)*10,600+abs(30-step)*5);time.sleep(.012)
                x('windowsize',window,900,600);x('windowmove',window,40,50)
                notify_position(env['DISPLAY'],window,40,50,900,600)
                time.sleep(2)
                def state():
                    fields=Path(f'/proc/{app.pid}/stat').read_text().rsplit(')',1)[1].split()
                    status=dict(line.split(':',1) for line in Path(f'/proc/{app.pid}/status').read_text().splitlines())
                    return int(fields[11])+int(fields[12]),int(status['VmRSS'].split()[0])
                before,_=state();time.sleep(2);after,rss=state()
                metrics={'binary_sha256':fingerprint,'backend':'Xvfb / native Cairo','idle_seconds':2,'idle_cpu_ticks':after-before,'rss_kib':rss}
                rollup = dict(line.split(':', 1) for line in Path(f'/proc/{app.pid}/smaps_rollup').read_text().splitlines()[1:])
                metrics['memory_kib'] = {name: int(rollup[name].split()[0]) for name in
                                         ('Rss', 'Pss', 'Private_Clean', 'Private_Dirty', 'Anonymous', 'Swap')}
                assert after-before<=2, f'Idle animation did not settle: {after-before} CPU ticks'
                key('ctrl+q');app.wait(timeout=10);assert app.returncode==0
                saved=json.loads((notes/'session.json').read_text())
                (output/'session.json').write_text(json.dumps(saved,indent=2)+'\n')
                assert saved['views'][str(first)]['wrap'] is True,'Command palette wrap action failed'
                assert saved['views'][str(created)]['wrap'] is True,'Wrap changed across tab switches'
                assert saved['views'][str(long_note)]['y']>1000,'Scrollbar position was not retained'
                assert saved['views'][str(long_note)]['caret']==0,'Scrollbar drag moved the caret'
                assert saved['position']==[40,50],'Window position was not saved'
                assert saved['titles'][str(created)]=='Ideas','Inline rename was not saved'
                assert saved['tabs'][:2]==[str(first),str(created)],'Tab order was not restored after dragging'
                app,window=start()
                shot('11-restored', 'Restart restores tab order, cursor, scroll, wrap and window placement.')
                type_text('Saved immediately. ');key('ctrl+q');app.wait(timeout=10)
                assert app.returncode==0
                assert long_note.read_text().startswith('Saved immediately. Line 000'),'Restart or immediate close lost the last edit'
                resize=sorted(int(line.split('=',1)[1])/1000 for line in (output/'native.log').read_text().splitlines() if line.startswith('resize_frame_us='))
                if resize:metrics['resize_event_to_swap_ms']={'samples':len(resize),'p50':resize[len(resize)//2],'p95':resize[min(len(resize)-1,int(len(resize)*.95))],'max':max(resize)}
                (output/'metrics.json').write_text(json.dumps(metrics,indent=2)+'\n')
                print('PASS: original layout, animated fire, typing, clipboard, undo/redo, rename, tabs, search, commands, native file opening, raw Markdown, wrap, scrollbar, resize, idle, session restoration and save-before-close.',flush=True)
                print(json.dumps(metrics),flush=True)
            except Exception:
                path=output/'failure.png'
                ImageGrab.grab(xdisplay=env['DISPLAY']).save(path);screenshots.append(path)
                (output/'failure-files.json').write_text(json.dumps({str(p.relative_to(directory)):p.read_text() for p in directory.rglob('*') if p.is_file() and p.suffix in ['.md','.json']},indent=2))
                raise
            finally:
                if app and app.poll() is None:
                    app.terminate()
                    app.wait(timeout=5)
                display.terminate()
                display.wait(timeout=5)
                send_gallery()

if __name__ == '__main__':
    main()
