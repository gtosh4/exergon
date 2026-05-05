#!/usr/bin/env python3
"""Generate placeholder GLB dev assets for Exergon.

Outputs to assets/models/{machines,cables,platforms,deposits}/.
No external dependencies — uses only stdlib.

Geometry matches what the Bevy code currently uses procedurally:
  machines        Cuboid(4, 4, 4)       per-machine base color
  platform        Cuboid(8, 0.25, 8)
  support column  Cuboid(0.5, 4, 0.5)
  power cable     Cylinder(r=0.04, h=1) — scale length at runtime
  logistics cable Cylinder(r=0.05, h=1)
  ore deposit     Sphere(r=0.5)
"""

import json
import math
import struct
from pathlib import Path


# ---------------------------------------------------------------------------
# Geometry builders — return (vertices, normals, indices)
# ---------------------------------------------------------------------------

def box_mesh(w: float, h: float, d: float):
    hw, hh, hd = w / 2, h / 2, d / 2
    # 6 faces × 4 verts each; winding = CCW from outside
    faces = [
        ((0,  1,  0), [(-hw,  hh, -hd), ( hw,  hh, -hd), ( hw,  hh,  hd), (-hw,  hh,  hd)]),
        ((0, -1,  0), [(-hw, -hh,  hd), ( hw, -hh,  hd), ( hw, -hh, -hd), (-hw, -hh, -hd)]),
        ((0,  0,  1), [(-hw, -hh,  hd), ( hw, -hh,  hd), ( hw,  hh,  hd), (-hw,  hh,  hd)]),
        ((0,  0, -1), [( hw, -hh, -hd), (-hw, -hh, -hd), (-hw,  hh, -hd), ( hw,  hh, -hd)]),
        (( 1,  0,  0), [( hw, -hh,  hd), ( hw, -hh, -hd), ( hw,  hh, -hd), ( hw,  hh,  hd)]),
        ((-1,  0,  0), [(-hw, -hh, -hd), (-hw, -hh,  hd), (-hw,  hh,  hd), (-hw,  hh, -hd)]),
    ]
    verts, norms, idxs = [], [], []
    for normal, corners in faces:
        base = len(verts)
        for c in corners:
            verts.append(c)
            norms.append(normal)
        idxs.extend([base, base + 1, base + 2, base, base + 2, base + 3])
    return verts, norms, idxs


def sphere_mesh(radius: float, lat_segs: int = 12, lon_segs: int = 16):
    verts, norms, idxs = [], [], []

    for lat in range(lat_segs + 1):
        theta = math.pi * lat / lat_segs
        sin_t, cos_t = math.sin(theta), math.cos(theta)
        for lon in range(lon_segs + 1):
            phi = 2 * math.pi * lon / lon_segs
            nx = sin_t * math.cos(phi)
            ny = cos_t
            nz = sin_t * math.sin(phi)
            verts.append((radius * nx, radius * ny, radius * nz))
            norms.append((nx, ny, nz))

    stride = lon_segs + 1
    for lat in range(lat_segs):
        for lon in range(lon_segs):
            a = lat * stride + lon
            b = a + 1
            c = a + stride
            d = c + 1
            if lat != 0:
                idxs.extend([a, c, b])
            if lat != lat_segs - 1:
                idxs.extend([b, c, d])

    return verts, norms, idxs


# ---------------------------------------------------------------------------
# GLB writer
# ---------------------------------------------------------------------------

def _pad4(data: bytes, pad_byte: int = 0) -> bytes:
    r = len(data) % 4
    return data + bytes([pad_byte] * ((4 - r) % 4))


def build_glb(verts, norms, idxs, color: list[float]) -> bytes:
    pos_bin = b"".join(struct.pack("3f", *v) for v in verts)
    nor_bin = b"".join(struct.pack("3f", *n) for n in norms)
    idx_bin = b"".join(struct.pack("I", i) for i in idxs)

    pos_bin = _pad4(pos_bin)
    nor_bin = _pad4(nor_bin)
    idx_bin = _pad4(idx_bin)

    bin_blob = pos_bin + nor_bin + idx_bin

    xs = [v[0] for v in verts]
    ys = [v[1] for v in verts]
    zs = [v[2] for v in verts]

    gltf = {
        "asset": {"version": "2.0", "generator": "exergon-gen-assets"},
        "scene": 0,
        "scenes": [{"nodes": [0]}],
        "nodes": [{"mesh": 0}],
        "meshes": [{"primitives": [{"attributes": {"POSITION": 0, "NORMAL": 1}, "indices": 2, "material": 0}]}],
        "accessors": [
            {
                "bufferView": 0, "componentType": 5126, "count": len(verts),
                "type": "VEC3",
                "min": [min(xs), min(ys), min(zs)],
                "max": [max(xs), max(ys), max(zs)],
            },
            {"bufferView": 1, "componentType": 5126, "count": len(norms), "type": "VEC3"},
            {"bufferView": 2, "componentType": 5125, "count": len(idxs),  "type": "SCALAR"},
        ],
        "bufferViews": [
            {"buffer": 0, "byteOffset": 0,                              "byteLength": len(pos_bin)},
            {"buffer": 0, "byteOffset": len(pos_bin),                  "byteLength": len(nor_bin)},
            {"buffer": 0, "byteOffset": len(pos_bin) + len(nor_bin),   "byteLength": len(idx_bin)},
        ],
        "buffers": [{"byteLength": len(bin_blob)}],
        "materials": [{
            "pbrMetallicRoughness": {
                "baseColorFactor": [*color, 1.0],
                "metallicFactor": 0.1,
                "roughnessFactor": 0.8,
            }
        }],
    }

    json_bytes = _pad4(json.dumps(gltf, separators=(",", ":")).encode(), pad_byte=0x20)

    json_chunk = struct.pack("<II", len(json_bytes), 0x4E4F534A) + json_bytes
    bin_chunk  = struct.pack("<II", len(bin_blob),  0x004E4942) + bin_blob
    header     = struct.pack("<III", 0x46546C67, 2, 12 + len(json_chunk) + len(bin_chunk))

    return header + json_chunk + bin_chunk


def write_glb(path: Path, verts, norms, idxs, color: list[float]):
    path.parent.mkdir(parents=True, exist_ok=True)
    data = build_glb(verts, norms, idxs, color)
    path.write_bytes(data)
    print(f"  {path.relative_to(path.parents[3])}  ({len(data):,} bytes)")


# ---------------------------------------------------------------------------
# Asset definitions
# ---------------------------------------------------------------------------

def main():
    root = Path(__file__).resolve().parents[1] / "assets" / "models"

    print("machines/")
    for name, color in [
        ("smelter",          [0.9,  0.45, 0.1 ]),
        ("assembler",        [0.2,  0.45, 0.9 ]),
        ("analysis_station", [0.1,  0.75, 0.55]),
        ("generator",        [0.9,  0.8,  0.1 ]),
        ("storage_crate",    [0.55, 0.6,  0.65]),
        ("refinery",         [0.65, 0.65, 0.65]),
        ("gateway",          [0.6,  0.1,  0.9 ]),
    ]:
        v, n, i = box_mesh(4.0, 4.0, 4.0)
        write_glb(root / "machines" / f"{name}.glb", v, n, i, color)

    print("platforms/")
    v, n, i = box_mesh(8.0, 0.25, 8.0)
    write_glb(root / "platforms" / "platform.glb", v, n, i, [0.5, 0.5, 0.55])
    v, n, i = box_mesh(0.5, 4.0, 0.5)
    write_glb(root / "platforms" / "support.glb",  v, n, i, [0.4, 0.4, 0.45])

    print("deposits/")
    v, n, i = sphere_mesh(0.5)
    write_glb(root / "deposits" / "ore_deposit.glb", v, n, i, [0.8, 0.6, 0.2])

    print("done.")


if __name__ == "__main__":
    main()
