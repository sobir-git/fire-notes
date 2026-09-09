#!/usr/bin/env python3
"""Read XRes resources and process/DRM accounting without changing server state.
Uses libXRes 1.2 ABI. Resource figures are server estimates, not physical RSS.
Sources: https://docs.kernel.org/filesystems/proc.html
https://docs.kernel.org/gpu/drm-usage-stats.html
XRes ABI: https://gitlab.freedesktop.org/xorg/lib/libxres/-/blob/master/include/X11/extensions/XRes.h
Run in a separate observer process: Xlib connection failure must not kill a probe.
"""

import argparse, ctypes as C, ctypes.util, json, os, time
from pathlib import Path

U = C.c_ulong
I = C.c_int
L = C.c_long
P = C.c_void_p


class Client(C.Structure):
    _fields_ = [("base", U), ("mask", U)]


class Type(C.Structure):
    _fields_ = [("atom", U), ("count", C.c_uint)]


class Spec(C.Structure):
    _fields_ = [("client", U), ("mask", C.c_uint)]


class Value(C.Structure):
    _fields_ = [("spec", Spec), ("length", L), ("value", P)]


class Resource(C.Structure):
    _fields_ = [("resource", U), ("type", U)]


class Size(C.Structure):
    _fields_ = [("spec", Resource), ("bytes", L), ("ref_count", L), ("use_count", L)]


class SizeValue(C.Structure):
    _fields_ = [
        ("size", Size),
        ("num_cross_references", L),
        ("cross_references", C.POINTER(Size)),
    ]


x = C.CDLL(ctypes.util.find_library("X11"))
r = C.CDLL(ctypes.util.find_library("XRes"))


def api(lib, name, args, result=I):
    f = getattr(lib, name)
    f.argtypes = args
    f.restype = result
    return f


open_display = api(x, "XOpenDisplay", [C.c_char_p], P)
close = api(x, "XCloseDisplay", [P])
free = api(x, "XFree", [P])
atom_name = api(x, "XGetAtomName", [P, U], P)
version = api(r, "XResQueryVersion", [P, C.POINTER(I), C.POINTER(I)])
clients = api(r, "XResQueryClients", [P, C.POINTER(I), C.POINTER(C.POINTER(Client))])
ids = api(
    r,
    "XResQueryClientIds",
    [P, L, C.POINTER(Spec), C.POINTER(L), C.POINTER(C.POINTER(Value))],
)
pid_of = api(r, "XResGetClientPid", [C.POINTER(Value)])
ids_free = api(r, "XResClientIdsDestroy", [L, C.POINTER(Value)], None)
resources = api(
    r, "XResQueryClientResources", [P, U, C.POINTER(I), C.POINTER(C.POINTER(Type))]
)
pixmaps = api(r, "XResQueryClientPixmapBytes", [P, U, C.POINTER(U)])
resource_bytes = api(
    r,
    "XResQueryResourceBytes",
    [P, U, L, C.POINTER(Resource), C.POINTER(L), C.POINTER(C.POINTER(SizeValue))],
)
resource_free = api(r, "XResResourceSizeValuesDestroy", [L, C.POINTER(SizeValue)], None)
# Concurrent client exit must not let Xlib's default error handler terminate the observer.
errors = []


@C.CFUNCTYPE(I, P, P)
def error_handler(display, event):
    errors.append("X protocol error during concurrent resource query")
    return 0


api(x, "XSetErrorHandler", [P], P)(C.cast(error_handler, P))


def drm_identity(fields, fd):
    # Missing client IDs cannot safely be deduplicated; retain per-fd observations.
    return (
        fields.get("drm-pdev", fields.get("drm-driver", "?"))
        + ":"
        + fields.get("drm-client-id", "fd-" + str(fd))
    )


def process(pid):
    result = {"pid": pid}
    try:
        result["comm"] = Path(f"/proc/{pid}/comm").read_text().strip()
        fields = {
            line.split(":", 1)[0]: int(line.split(":", 1)[1].split()[0]) * 1024
            for line in Path(f"/proc/{pid}/smaps_rollup").read_text().splitlines()[1:]
        }
        result["memory_bytes"] = {
            k: fields[k]
            for k in [
                "Rss",
                "Pss",
                "Private_Clean",
                "Private_Dirty",
                "Shared_Clean",
                "Shared_Dirty",
                "Swap",
                "SwapPss",
            ]
            if k in fields
        }
    except OSError as e:
        result["memory_error"] = str(e)
    drm = {}
    try:
        for path in Path(f"/proc/{pid}/fdinfo").iterdir():
            try:
                fields = dict(
                    line.split(":", 1)
                    for line in path.read_text().splitlines()
                    if line.startswith("drm-")
                )
                if not fields:
                    continue
                fields = {k: v.strip() for k, v in fields.items()}
                key = drm_identity(fields, path.name)
                if key not in drm:
                    drm[key] = {"fields": fields, "fds": []}
                drm[key]["fds"].append(path.name)
            except OSError as e:
                result.setdefault("fd_errors", []).append(str(e))
    except OSError as e:
        result["drm_error"] = str(e)
    result["drm_clients"] = drm
    return result


def snapshot(display, pids, window):
    errors.clear()
    d = open_display(display.encode())
    assert d, "XOpenDisplay failed"
    try:
        major = I()
        minor = I()
        assert version(d, C.byref(major), C.byref(minor)), "XRes absent"
        assert (major.value, minor.value) >= (
            1,
            2,
        ), "XRes 1.2 required for PID attribution"
        n = I()
        values = C.POINTER(Client)()
        assert clients(d, C.byref(n), C.byref(values))
        ranges = [(v.base, v.mask) for v in values[: n.value]]
        free(values)
        out = []
        names = {}

        def name(atom):
            if atom not in names:
                ptr = atom_name(d, atom)
                names[atom] = (
                    C.string_at(ptr).decode(errors="replace") if ptr else str(atom)
                )
                if ptr:
                    free(ptr)
            return names[atom]

        for base, mask in ranges:
            if window is not None and window & ~mask != base:
                continue
            spec = Spec(base, 2)
            count = L()
            identified = C.POINTER(Value)()
            pid = None
            # XResQueryClientIds returns Success (0), unlike the v1.0 Status APIs.
            if ids(d, 1, C.byref(spec), C.byref(count), C.byref(identified)) == 0:
                for i in range(count.value):
                    candidate = pid_of(C.byref(identified[i]))
                    if candidate >= 0:
                        pid = candidate
                ids_free(count, identified)
            else:
                errors.append(f"XResQueryClientIds failed for {base}")
            if pids and pid not in pids:
                continue
            item = {"client_base": base, "client_mask": mask, "pid": pid}
            types = C.POINTER(Type)()
            nt = I()
            if resources(d, base, C.byref(nt), C.byref(types)):
                item["resource_counts"] = {
                    name(v.atom): v.count for v in types[: nt.value]
                }
                free(types)
            else:
                errors.append(f"XResQueryClientResources failed for {base}")
            total = U()
            if pixmaps(d, base, C.byref(total)):
                item["pixmap_bytes_estimate"] = total.value
            else:
                errors.append(f"XResQueryClientPixmapBytes failed for {base}")
            sizes = C.POINTER(SizeValue)()
            ns = L()
            all_resources = Resource(0, 0)
            if (
                resource_bytes(
                    d, base, 1, C.byref(all_resources), C.byref(ns), C.byref(sizes)
                )
                == 0
            ):
                entries = []

                def size(s):
                    return {
                        "resource": s.spec.resource,
                        "type": name(s.spec.type),
                        "bytes": s.bytes,
                        "ref_count": s.ref_count,
                        "use_count": s.use_count,
                    }

                for v in sizes[: ns.value]:
                    entries.append(
                        {
                            "size": size(v.size),
                            "cross_references": [
                                size(s)
                                for s in v.cross_references[: v.num_cross_references]
                            ],
                        }
                    )
                item["resource_bytes_estimates"] = entries
                resource_free(ns, sizes)
            else:
                errors.append(f"XResQueryResourceBytes failed for {base}")
            out.append(item)
        return {
            "timestamp_ns": time.time_ns(),
            "display": display,
            "xres_version": [major.value, minor.value],
            "xres_clients": out,
            "processes": [
                process(pid)
                for pid in sorted(pids or {i["pid"] for i in out if i["pid"]})
            ],
            "query_errors": errors,
            "limits": "XRes sizes are best-effort estimates; unsupported resources may be omitted or reported as zero. On this Xorg, GLYPHSET/WINDOW/PICTURE direct byte estimates returned zero; these are not proof of zero cost. Pixmap estimates included a composited window backing store in the controlled Cairo check. Cross-references overlap and must not be summed blindly. DRM fields preserve native units and deduplicate file descriptors by device/client ID; shared buffers can still overlap between distinct clients. Process/server/DRM bytes are not additive.",
        }
    finally:
        close(d)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--display", default=os.environ.get("DISPLAY", ":0"))
    parser.add_argument("--pid", type=int, action="append", default=[])
    parser.add_argument("--window", type=lambda x: int(x, 0))
    parser.add_argument("--output")
    args = parser.parse_args()
    data = (
        json.dumps(snapshot(args.display, set(args.pid), args.window), indent=2) + "\n"
    )
    if args.output:
        Path(args.output).write_text(data)
    else:
        print(data, end="")
