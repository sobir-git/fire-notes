#!/usr/bin/env python3
"""Measure guarded native requests to observed app drawable pixels, never scanout.

Calibration determines target pixels before timed trials. XGetImage reads only
Fire Notes' drawable; no root screenshot or command acknowledgement is a frame.
"""
import argparse
import ctypes as C
import hashlib
import json
import os
from pathlib import Path
import socket
import subprocess
import sys
import time

from PIL import Image
from memory_probe import desktop_state, fixture, wait_until
from native_input import NativeInput
from private_session import PrivateSession


class XImage(C.Structure):
    _fields_ = [('width', C.c_int), ('height', C.c_int), ('xoffset', C.c_int),
                ('format', C.c_int), ('data', C.c_void_p), ('byte_order', C.c_int),
                ('bitmap_unit', C.c_int), ('bitmap_bit_order', C.c_int),
                ('bitmap_pad', C.c_int), ('depth', C.c_int), ('bytes_per_line', C.c_int),
                ('bits_per_pixel', C.c_int), ('red_mask', C.c_ulong),
                ('green_mask', C.c_ulong), ('blue_mask', C.c_ulong)]


class Observer(NativeInput):
    def __init__(self, display):
        super().__init__(display)
        def bind(lib, name, args, result=C.c_int):
            fn = getattr(lib, name); fn.argtypes = args; fn.restype = result
            return fn
        D, W, I = C.c_void_p, C.c_ulong, C.c_int
        self.get_image = bind(self.x, 'XGetImage', [D, W, I, I, C.c_uint, C.c_uint, W, I], C.POINTER(XImage))
        self.destroy_image = bind(self.x, 'XDestroyImage', [C.POINTER(XImage)])
        self.translate = bind(self.x, 'XTranslateCoordinates', [D, W, W, I, I, C.POINTER(I), C.POINTER(I), C.POINTER(W)])
        self.motion = bind(self.t, 'XTestFakeMotionEvent', [D, I, I, I, W])
        self.resize = bind(self.x, 'XResizeWindow', [D, W, C.c_uint, C.c_uint])
        self.flush = bind(self.x, 'XFlush', [D])

    def image(self, window, box):
        x, y, width, height = box
        ptr = self.get_image(self.display, int(window), x, y, width, height, C.c_ulong(-1).value, 2)
        if not ptr:
            raise RuntimeError('XGetImage failed; drawable must be visible and region in bounds')
        observed = time.monotonic_ns()
        try:
            data = ptr.contents
            if (data.bits_per_pixel, data.byte_order, data.red_mask, data.green_mask, data.blue_mask) != (32, 0, 0xff0000, 0xff00, 0xff):
                raise RuntimeError('Unsupported XImage format; probe requires little-endian 32-bit TrueColor')
            raw = C.string_at(data.data, data.bytes_per_line * height)
            frame = Image.frombytes('RGB', (width, height), raw, 'raw', 'BGRX', data.bytes_per_line, 1)
            return frame, observed
        finally:
            self.destroy_image(ptr)

    def move(self, window, position):
        rx, ry, child = C.c_int(), C.c_int(), C.c_ulong()
        self.grab(self.display)
        try:
            self.guard(window)
            if not self.translate(self.display, int(window), self.root(self.display), *position, C.byref(rx), C.byref(ry), C.byref(child)):
                raise RuntimeError('Cannot translate target coordinates')
            started = time.monotonic_ns()
            if not self.motion(self.display, -1, rx.value, ry.value, 0):
                raise RuntimeError('XTest pointer motion failed')
        finally:
            self.ungrab(self.display)
            self.flush(self.display)
        return started

    def resize_request(self, window, size):
        self.grab(self.display)
        try:
            self.guard(window)
            started = time.monotonic_ns()
            self.resize(self.display, int(window), *size)
        finally:
            self.ungrab(self.display)
            self.flush(self.display)
        return started


def request(path, **payload):
    with socket.socket(socket.AF_UNIX) as connection:
        connection.settimeout(5)
        connection.connect(str(path))
        connection.sendall(json.dumps(payload).encode() + b'\n')
        with connection.makefile('r') as stream:
            result = json.loads(stream.readline())
    if 'error' in result:
        raise RuntimeError(result['error'])
    return result


def targets(first, second, box):
    """Select stable response pixels from independent pre-trial calibration."""
    x, y, width, height = box
    changed = [(xx, yy) for yy in range(y, y+height) for xx in range(x, x+width)
               if first.getpixel((xx, yy)) != second.getpixel((xx, yy))]
    if len(changed) < 8:
        raise AssertionError('Calibration produced fewer than eight changed target pixels')
    positions = [changed[i * (len(changed)-1)//7] for i in range(8)]
    return [[[xx, yy, list(frame.getpixel((xx, yy)))] for xx, yy in positions] for frame in (first, second)]


def file_record(path, output):
    return {'path': str(path.relative_to(output)), 'sha256': hashlib.sha256(path.read_bytes()).hexdigest()}


def check_desktop(env):
    state = desktop_state(env)
    if not state.get('available') or state.get('locked'):
        raise RuntimeError(f'Desktop lock state unavailable or locked; no app launched/input sent: {state}')
    return state


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary', default='target/release/fire-notes')
    parser.add_argument('--output', required=True, help='New artifact directory (must not exist)')
    parser.add_argument('--physical-display', action='store_true', help='Optional existing X11 desktop; requires unlocked session and restores focus')
    args = parser.parse_args()
    # Check the real session before activating isolated D-Bus services or launching anything.
    real_env = dict(os.environ)
    state = {'available': True, 'kind': 'private_x11'} if not args.physical_display else check_desktop(real_env)
    binary = Path(args.binary).resolve()
    output = Path(args.output).resolve(); output.mkdir(parents=True, exist_ok=False)
    with binary.open('rb') as stream: os.fsync(stream.fileno())
    report = {'schema': 1, 'probe_sha256': hashlib.sha256(Path(__file__).read_bytes()).hexdigest(), 'binary_sha256': hashlib.sha256(binary.read_bytes()).hexdigest(),
              'display_kind': 'private_x11' if not args.physical_display else 'physical_x11',
              'desktop_state': state, 'metric': 'native_input_to_drawable_pixels',
              'temporary_data': True, 'complete': False, 'samples': {}, 'calibration': {},
              'workload': {'tabs': 3, 'bytes_per_document': 8192, 'physical_window_sizes': [[1200, 800], [1240, 800]], 'samples_per_state': 20},
              'measurement_limits': [
                  'XGetImage observes the app drawable on the X server, not compositor presentation, scanout or physical photons.',
                  'Input starts immediately before XTestFakeMotionEvent or XResizeWindow; includes client/server transport, scheduling and polling overhead.',
                  'Resize is a programmatic native XResizeWindow request, not a user border drag.',
                  'Observer timestamps follow XGetImage completion. PNG encoding and full app frame capture occur after the timed observation.',
                  'Header target colors are calibrated before trials and outside the animated body. This does not measure editor selection-drag latency.',
                  'The opt-in inspection service sets up/checks states outside timed intervals; production runs normally omit it.',
                  'XGetImage requires visible in-bounds regions; obscured content can come from backing storage or be undefined. Desktop focus is guarded, but scanout/occlusion is not independently established.'],
              'primary_source': 'https://xorg.freedesktop.org/archive/X11R7.5/doc/man/man3/XGetImage.3.html'}
    app = display = None
    previous_focus = None
    try:
        with PrivateSession() as session, (output/'native.log').open('w') as log:
            try:
                if not args.physical_display:
                    with (session.directory/'display').open('w+') as number:
                        display = subprocess.Popen(['Xvfb', '-displayfd', str(number.fileno()), '-screen', '0', '2560x1800x24', '-nolisten', 'tcp'], pass_fds=(number.fileno(),), stdout=log, stderr=log)
                        def ready():
                            number.seek(0); return number.read().strip()
                        wait_until(ready, 'Private X server did not start')
                        display_name = ':' + ready()
                else:
                    display_name = real_env['DISPLAY']
                    authority = real_env.get('XAUTHORITY') or str(Path(real_env['HOME'])/'.Xauthority')
                    if Path(authority).is_file(): session.env['XAUTHORITY'] = authority
                env = session.activate(display_name)
                env['WINIT_X11_SCALE_FACTOR'] = '2'
                env.pop('FIRE_UI_PROFILE', None)
                sock = session.directory/'ui.sock'; env['FIRE_UI_INSPECT'] = str(sock)
                data = session.directory/'notes'; data.mkdir()
                text, _ = fixture(data, 3, 8192)
                report['workload']['fixture_sha256'] = hashlib.sha256(text.encode()).hexdigest()
                def x(*args):
                    return subprocess.check_output(['xdotool', *map(str, args)], env=env, text=True, stderr=subprocess.DEVNULL).strip()
                if args.physical_display: previous_focus = x('getwindowfocus')
                app = subprocess.Popen([str(binary), '--data-dir', str(data)], env=env, stdout=log, stderr=log)
                wait_until(lambda: sock.exists() or app.poll() is not None, 'Inspection socket did not appear')
                if app.poll() is not None: raise RuntimeError('Notes exited before inspection was ready')
                window = x('search', '--pid', app.pid, '--name', '^Fire Notes$').splitlines()[0]
                x('windowraise', window); x('windowfocus', window)
                x('windowsize', window, 1200, 800)
                time.sleep(.4)
                def geometry():
                    g = dict(line.split('=', 1) for line in x('getwindowgeometry', '--shell', window).splitlines())
                    return int(g['WIDTH']), int(g['HEIGHT'])
                snapshot = request(sock)
                scale = geometry()[0] / snapshot['size']['width']
                button = next(n for n in snapshot['nodes'] if n['label'] == 'New note')['bounds']
                hover = (round((button['x']+button['width']/2)*scale), round((button['y']+button['height']/2)*scale))
                away = (40, 140)
                header_height = round(40*scale)
                def desktop_guard():
                    if args.physical_display: check_desktop(real_env)
                with Observer(display_name) as observer:
                    def frame():
                        return observer.image(window, (0, 0, *geometry()))[0]
                    observer.move(window, away)
                    request(sock, key='note-body', action='set_selection', anchor=0, caret=0)
                    request(sock, label='New note', action='focus')
                    time.sleep(1.5)
                    hover_box = (max(0, round(button['x']*scale)), max(0, round(button['y']*scale)), round(button['width']*scale), round(button['height']*scale))
                    def sample(name, index, expected, perform, input_kind):
                        desktop_guard(); observer.guard(window)
                        before = frame()
                        if not any(list(before.getpixel((px, py))) != color for px, py, color in expected):
                            raise AssertionError('Response pixels already present before input')
                        left=min(p[0] for p in expected); top=min(p[1] for p in expected)
                        box=(left, top, max(p[0] for p in expected)-left+1, max(p[1] for p in expected)-top+1)
                        started=perform(); deadline=started+2_000_000_000
                        polls=0
                        while True:
                            region, observed=observer.image(window, box); polls+=1
                            actual=[[px, py, list(region.getpixel((px-left, py-top)))] for px, py, _ in expected]
                            if actual == expected: break
                            if observed > deadline: raise TimeoutError(f'{name} response pixels not observed')
                            time.sleep(.0005)
                        after=frame()
                        if any(list(after.getpixel((px, py))) != color for px, py, color in expected):
                            raise AssertionError('Full app frame no longer contains observed response')
                        before_path=output/f'{name}-{index:02}-before.png'; after_path=output/f'{name}-{index:02}-after.png'
                        before.save(before_path); after.save(after_path)
                        report['samples'][name].append({'input_monotonic_ns': started, 'observed_monotonic_ns': observed,
                            'milliseconds': (observed-started)/1_000_000, 'input_kind': input_kind, 'polls': polls,
                            'expected_pixels': expected, 'observed_pixels': actual,
                            'before_frame': file_record(before_path, output), 'frame': file_record(after_path, output)})
                    for name in ('pointer_idle', 'pointer_active_fire'):
                        report['samples'][name]=[]
                        observer.move(window, away); time.sleep(.1)
                        if name == 'pointer_active_fire':
                            request(sock, key='note-body', action='focus')
                            request(sock, key='note-body', action='set_selection', anchor=0, caret=len(text[:120].encode()))
                            time.sleep(.15)
                            if not request(sock)['animating']: raise AssertionError('Selected fire is not active')
                        else:
                            if request(sock)['animating']: raise AssertionError('Idle pointer trial has active animation')
                        # Each focus/animation state gets its own independent calibration.
                        hover_frames = []
                        for pos in (away, hover):
                            observer.move(window, pos); time.sleep(.15); hover_frames.append(frame())
                        hover_targets = targets(*hover_frames, hover_box)
                        report['calibration'][name] = []
                        for i, f in enumerate(hover_frames):
                            path = output/f'calibration-{name}-{i}.png'; f.save(path)
                            report['calibration'][name].append({'frame': file_record(path, output), 'target_pixels': hover_targets[i]})
                        observer.move(window, away); time.sleep(.1)
                        for index in range(20):
                            target=1 if index%2 == 0 else 0
                            sample(name, index, hover_targets[target], lambda target=target: observer.move(window, (away, hover)[target]), 'XTestFakeMotionEvent')
                            if name == 'pointer_active_fire' and not request(sock)['animating']:
                                raise AssertionError('Fire stopped during active-fire trial')
                    request(sock, key='note-body', action='set_selection', anchor=0, caret=0)
                    request(sock, label='New note', action='focus')
                    observer.move(window, away); time.sleep(1.5)
                    sizes=((1200,800),(1240,800)); resize_frames=[]
                    for size in sizes:
                        observer.resize_request(window,size); time.sleep(.2); resize_frames.append(frame())
                    resize_targets=targets(*resize_frames,(1050,0,150,header_height))
                    report['calibration']['resize'] = []
                    for i, f in enumerate(resize_frames):
                        path = output/f'calibration-resize-{i}.png'; f.save(path)
                        report['calibration']['resize'].append({'frame': file_record(path, output), 'target_pixels': resize_targets[i]})
                    report['samples']['resize']=[]
                    for index in range(20):
                        target=index%2
                        sample('resize',index,resize_targets[target],lambda target=target: observer.resize_request(window,sizes[target]),'XResizeWindow')
                report['complete']=True
            finally:
                if app and app.poll() is None: app.terminate(); app.wait(timeout=8)
                if previous_focus:
                    subprocess.run(['xdotool','windowfocus',previous_focus],env=env,check=True,timeout=5)
                if display: display.terminate(); display.wait(timeout=5)
    except Exception as error:
        report['complete']=False
        report['error']=repr(error)
    finally:
        (output/'results.json').write_text(json.dumps(report,indent=2)+'\n')
    print(json.dumps({'complete': report['complete'], 'report':str(output/'results.json'), 'error':report.get('error')}))
    return 0 if report['complete'] else 1


if __name__ == '__main__':
    sys.exit(main())
