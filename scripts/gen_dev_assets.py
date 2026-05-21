#!/usr/bin/env python3
"""Generate placeholder GLB dev assets for Exergon.

Outputs to assets/models/{machines,platforms,deposits}/.
No external dependencies — uses only stdlib.

Machine GLBs contain a body mesh plus one named child node per IO port:
  Port_Energy_<i>      yellow stub, sphere collider in code
  Port_Logistics_<i>   green stub,  sphere collider in code

The Bevy loader reads `Gltf.named_nodes`, walks `Port_*`, and extracts
per-port local-space transforms — replacing the previous hardcoded
`MachineTierDef.{energy,logistics}_io_offsets` lists in RON.
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
    faces = [
        ((0,  1,  0), [(-hw,  hh,  hd), ( hw,  hh,  hd), ( hw,  hh, -hd), (-hw,  hh, -hd)]),
        ((0, -1,  0), [(-hw, -hh, -hd), ( hw, -hh, -hd), ( hw, -hh,  hd), (-hw, -hh,  hd)]),
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
# GLB writer — supports multi-node hierarchies, multi-mesh, multi-material
# ---------------------------------------------------------------------------

def _pad4(data: bytes, pad_byte: int = 0) -> bytes:
    r = len(data) % 4
    return data + bytes([pad_byte] * ((4 - r) % 4))


class GlbBuilder:
    """Accumulates meshes, materials, and nodes; emits a single .glb."""

    def __init__(self):
        self.bin_chunks: list[bytes] = []
        self.bin_offset = 0
        self.accessors: list[dict] = []
        self.buffer_views: list[dict] = []
        self.meshes: list[dict] = []
        self.materials: list[dict] = []
        self.nodes: list[dict] = []
        self.root_children: list[int] = []

    def add_material(self, color: list[float], metallic: float = 0.1, roughness: float = 0.8,
                     unlit: bool = False) -> int:
        idx = len(self.materials)
        mat = {
            "pbrMetallicRoughness": {
                "baseColorFactor": [*color, 1.0],
                "metallicFactor": metallic,
                "roughnessFactor": roughness,
            }
        }
        if unlit:
            mat["extensions"] = {"KHR_materials_unlit": {}}
        self.materials.append(mat)
        return idx

    def _add_buffer_view(self, data: bytes) -> int:
        idx = len(self.buffer_views)
        self.buffer_views.append({
            "buffer": 0,
            "byteOffset": self.bin_offset,
            "byteLength": len(data),
        })
        self.bin_chunks.append(data)
        self.bin_offset += len(data)
        return idx

    def add_mesh(self, verts, norms, idxs, material_idx: int) -> int:
        pos_bin = _pad4(b"".join(struct.pack("3f", *v) for v in verts))
        nor_bin = _pad4(b"".join(struct.pack("3f", *n) for n in norms))
        idx_bin = _pad4(b"".join(struct.pack("I", i) for i in idxs))

        pos_view = self._add_buffer_view(pos_bin)
        nor_view = self._add_buffer_view(nor_bin)
        idx_view = self._add_buffer_view(idx_bin)

        xs = [v[0] for v in verts]
        ys = [v[1] for v in verts]
        zs = [v[2] for v in verts]

        pos_acc = len(self.accessors)
        self.accessors.append({
            "bufferView": pos_view, "componentType": 5126, "count": len(verts),
            "type": "VEC3",
            "min": [min(xs), min(ys), min(zs)],
            "max": [max(xs), max(ys), max(zs)],
        })
        nor_acc = len(self.accessors)
        self.accessors.append({
            "bufferView": nor_view, "componentType": 5126, "count": len(norms), "type": "VEC3",
        })
        idx_acc = len(self.accessors)
        self.accessors.append({
            "bufferView": idx_view, "componentType": 5125, "count": len(idxs), "type": "SCALAR",
        })

        mesh_idx = len(self.meshes)
        self.meshes.append({
            "primitives": [{
                "attributes": {"POSITION": pos_acc, "NORMAL": nor_acc},
                "indices": idx_acc,
                "material": material_idx,
            }]
        })
        return mesh_idx

    def add_node(self, name: str | None, mesh_idx: int | None,
                 translation: tuple[float, float, float] | None = None) -> int:
        idx = len(self.nodes)
        node: dict = {}
        if name is not None:
            node["name"] = name
        if mesh_idx is not None:
            node["mesh"] = mesh_idx
        if translation is not None and translation != (0.0, 0.0, 0.0):
            node["translation"] = list(translation)
        self.nodes.append(node)
        self.root_children.append(idx)
        return idx

    def build(self) -> bytes:
        bin_blob = b"".join(self.bin_chunks)

        # Single root node groups everything so Bevy spawns a single SceneRoot
        # tree. Children list = every other node by index.
        scene_nodes = [len(self.nodes)]
        root_idx = len(self.nodes)
        self.nodes.append({"name": "Root", "children": self.root_children})

        extensions_used: list[str] = []
        for m in self.materials:
            if "extensions" in m and "KHR_materials_unlit" in m["extensions"]:
                extensions_used.append("KHR_materials_unlit")
                break

        gltf = {
            "asset": {"version": "2.0", "generator": "exergon-gen-assets"},
            "scene": 0,
            "scenes": [{"nodes": scene_nodes}],
            "nodes": self.nodes,
            "meshes": self.meshes,
            "accessors": self.accessors,
            "bufferViews": self.buffer_views,
            "buffers": [{"byteLength": len(bin_blob)}],
            "materials": self.materials,
        }
        if extensions_used:
            gltf["extensionsUsed"] = extensions_used

        json_bytes = _pad4(json.dumps(gltf, separators=(",", ":")).encode(), pad_byte=0x20)

        json_chunk = struct.pack("<II", len(json_bytes), 0x4E4F534A) + json_bytes
        bin_chunk = struct.pack("<II", len(bin_blob), 0x004E4942) + bin_blob
        header = struct.pack("<III", 0x46546C67, 2, 12 + len(json_chunk) + len(bin_chunk))

        return header + json_chunk + bin_chunk


def write_glb(path: Path, data: bytes):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(data)
    print(f"  {path.relative_to(path.parents[3])}  ({len(data):,} bytes)")


# ---------------------------------------------------------------------------
# Machine port layouts (canonical-space offsets, pre-orientation)
# ---------------------------------------------------------------------------

ENERGY_PORT_COLOR = [1.0, 0.85, 0.0]
LOGISTICS_PORT_COLOR = [0.1, 0.9, 0.2]
PORT_STUB_SIZE = 0.8

MACHINES = {
    "smelter": {
        "color": [0.9, 0.45, 0.1],
        "energy":    [(3, 0, 0)],
        "logistics": [(0, 0, 3), (0, 0, -3), (-3, 0, 0), (-3, 0, 2)],
    },
    "assembler": {
        "color": [0.2, 0.45, 0.9],
        "energy":    [(3, 0, 0), (3, 0, -2)],
        "logistics": [(0, 0, 3), (0, 0, -3), (-3, 0, 0), (-3, 0, 2), (-3, 0, -2)],
    },
    "analysis_station": {
        "color": [0.1, 0.75, 0.55],
        "energy":    [(3, 0, 0)],
        "logistics": [(0, 0, 3), (0, 0, -3), (-3, 0, 0)],
    },
    "generator": {
        "color": [0.9, 0.8, 0.1],
        "energy":    [(0, 0, 3), (0, 0, -3), (3, 0, 0), (-3, 0, 0)],
        "logistics": [],
    },
    "storage_crate": {
        "color": [0.55, 0.6, 0.65],
        "energy":    [],
        "logistics": [(3, 0, 0), (-3, 0, 0), (0, 0, 3), (0, 0, -3)],
    },
    "refinery": {
        "color": [0.65, 0.65, 0.65],
        "energy":    [(3, 0, 0), (3, 0, 2), (3, 0, -2)],
        "logistics": [(0, 0, 3), (0, 0, -3), (-3, 0, 0), (-3, 0, 2)],
    },
    "gateway": {
        "color": [0.6, 0.1, 0.9],
        "energy":    [(3, 0, 0), (-3, 0, 0)],
        "logistics": [(0, 0, 3), (0, 0, -3), (3, 0, 2), (-3, 0, 2)],
    },
    "solar_generator": {
        "color": [0.15, 0.55, 0.95],
        "energy":    [(3, 0, 0), (-3, 0, 0), (0, 0, 3), (0, 0, -3)],
        "logistics": [],
    },
    "combustion_generator": {
        "color": [0.7, 0.35, 0.1],
        "energy":    [(3, 0, 0), (-3, 0, 0)],
        "logistics": [(0, 0, 3), (0, 0, -3)],
    },
}


def build_machine_glb(spec: dict) -> bytes:
    g = GlbBuilder()
    body_mat = g.add_material(spec["color"])
    energy_mat = g.add_material(ENERGY_PORT_COLOR, metallic=0.0, roughness=0.4, unlit=True)
    logistics_mat = g.add_material(LOGISTICS_PORT_COLOR, metallic=0.0, roughness=0.4, unlit=True)

    bv, bn, bi = box_mesh(4.0, 4.0, 4.0)
    body_mesh = g.add_mesh(bv, bn, bi, body_mat)

    pv, pn, pi = box_mesh(PORT_STUB_SIZE, PORT_STUB_SIZE, PORT_STUB_SIZE)
    energy_mesh = g.add_mesh(pv, pn, pi, energy_mat)
    logistics_mesh = g.add_mesh(pv, pn, pi, logistics_mat)

    g.add_node("Body", body_mesh)
    for i, off in enumerate(spec["energy"]):
        g.add_node(f"Port_Energy_{i}", energy_mesh, (float(off[0]), float(off[1]), float(off[2])))
    for i, off in enumerate(spec["logistics"]):
        g.add_node(f"Port_Logistics_{i}", logistics_mesh, (float(off[0]), float(off[1]), float(off[2])))

    return g.build()


def build_simple_glb(verts, norms, idxs, color: list[float], name: str = "Root") -> bytes:
    g = GlbBuilder()
    mat = g.add_material(color)
    mesh = g.add_mesh(verts, norms, idxs, mat)
    g.add_node(name, mesh)
    return g.build()


# ---------------------------------------------------------------------------
# Entry point
# ---------------------------------------------------------------------------

def main():
    root = Path(__file__).resolve().parents[1] / "assets" / "models"

    print("machines/")
    for name, spec in MACHINES.items():
        write_glb(root / "machines" / f"{name}.glb", build_machine_glb(spec))

    print("platforms/")
    v, n, i = box_mesh(8.0, 0.25, 8.0)
    write_glb(root / "platforms" / "platform.glb", build_simple_glb(v, n, i, [0.5, 0.5, 0.55], "Platform"))
    v, n, i = box_mesh(0.5, 4.0, 0.5)
    write_glb(root / "platforms" / "support.glb", build_simple_glb(v, n, i, [0.4, 0.4, 0.45], "Support"))

    print("deposits/")
    v, n, i = sphere_mesh(0.5)
    write_glb(root / "deposits" / "ore_deposit.glb", build_simple_glb(v, n, i, [0.8, 0.6, 0.2], "OreDeposit"))

    print("done.")


if __name__ == "__main__":
    main()
