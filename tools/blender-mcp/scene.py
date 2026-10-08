"""BLD-01: runs inside Blender. server.py prepends this text to every script."""

import bpy
import math
import os
from mathutils import Vector


CAM_DIR = (0.35, -1.0, 0.45)  # from the front (-Y), a little to the right and above
CAM_AZ = math.degrees(math.atan2(CAM_DIR[0], -CAM_DIR[1]))  # turn an object by this to face the camera


def prepare():
    for o in list(bpy.data.objects):
        bpy.data.objects.remove(o, do_unlink=True)
    if bpy.context.scene.world is None:
        bpy.context.scene.world = bpy.data.worlds.new("World")
    world = bpy.context.scene.world
    if world.node_tree is None:
        world.use_nodes = True
    bg_node = world.node_tree.nodes["Background"]
    bg_node.inputs[0].default_value = (0.05, 0.05, 0.06, 1)


def color(obj, rgb):
    """Give obj one coloured material: rgb is (r, g, b) or (r, g, b, a), each 0..1."""
    rgba = tuple(rgb) + (1.0,) if len(rgb) == 3 else tuple(rgb)
    mat = bpy.data.materials.new(obj.name + "-color")
    if mat.node_tree is None:
        mat.use_nodes = True
    mat.node_tree.nodes["Principled BSDF"].inputs["Base Color"].default_value = rgba
    mat.diffuse_color = rgba
    obj.data.materials.clear()
    obj.data.materials.append(mat)
    return mat


def _bounds():
    valid_types = {"MESH", "CURVE", "SURFACE", "META", "FONT"}
    corners = []
    objs = [o for o in bpy.data.objects if o.type in valid_types and not o.get("kk_ground")]
    for o in objs or [o for o in bpy.data.objects if o.type in valid_types]:
        if o.bound_box:
            for c in o.bound_box:
                corners.append(o.matrix_world @ Vector(c))
    if not corners:
        return Vector((0, 0, 0)), 2.0
    lo = Vector([min(c[i] for c in corners) for i in range(3)])
    hi = Vector([max(c[i] for c in corners) for i in range(3)])
    center = (lo + hi) / 2
    size = max(hi - lo)
    return center, max(size, 1.0)


def finish(out, width=640, height=480, engine="EEVEE"):
    scene = bpy.context.scene
    if scene.camera is None:
        cams = [o for o in scene.objects if o.type == "CAMERA"]
        if cams:
            scene.camera = cams[0]
        else:
            center, size = _bounds()
            cam_obj = bpy.data.objects.new("Camera", bpy.data.cameras.new("Camera"))
            scene.collection.objects.link(cam_obj)
            cam_obj.location = center + Vector(CAM_DIR).normalized() * max(size * 2.5, 3.5)
            cam_obj.rotation_euler = (center - cam_obj.location).to_track_quat("-Z", "Y").to_euler()
            scene.camera = cam_obj
    if not any(o.type == "LIGHT" for o in scene.objects):
        sun_light = bpy.data.lights.new("Sun", type="SUN")
        sun_light.energy = 3.0
        sun_object = bpy.data.objects.new("Sun", sun_light)
        scene.collection.objects.link(sun_object)
        sun_object.rotation_euler = (math.radians(50), 0, math.radians(30))
    engine_upper = engine.upper()
    if engine_upper == "CYCLES":
        scene.render.engine = "CYCLES"
        scene.cycles.device = "CPU"
        scene.cycles.samples = int(os.environ.get("BLENDER_SAMPLES", "128"))
        scene.cycles.use_denoising = True
        scene.cycles.denoiser = "OPENIMAGEDENOISE"
    else:
        try:
            scene.render.engine = "BLENDER_EEVEE"
        except TypeError:
            scene.render.engine = "BLENDER_EEVEE_NEXT"
    scene.render.resolution_x = int(width)
    scene.render.resolution_y = int(height)
    scene.render.resolution_percentage = 100
    scene.render.image_settings.file_format = "PNG"
    scene.render.filepath = out
    # CHAT-05: the scene itself, next to the picture, so it can be opened and worked on in Blender.
    bpy.ops.wm.save_as_mainfile(filepath=os.path.splitext(out)[0] + ".blend", compress=True)
    bpy.ops.render.render(write_still=True)
    with open(os.path.splitext(out)[0] + ".txt", "w") as f:
        f.write(layout_report())
    print("rendered", out)


# ---- BLD-03: placement helpers. Objects get real contact (on, stacked, attached) instead of guessed coordinates.

_PAINT = color  # add() has a "color" argument of its own
SIDES = {"front": (0, -1, 0), "back": (0, 1, 0), "left": (-1, 0, 0), "right": (1, 0, 0), "top": (0, 0, 1), "bottom": (0, 0, -1)}
COLORS = {"red": (0.8, 0.05, 0.05), "orange": (0.95, 0.45, 0.05), "yellow": (0.95, 0.8, 0.1), "green": (0.1, 0.55, 0.15),
          "blue": (0.1, 0.25, 0.8), "purple": (0.45, 0.15, 0.7), "pink": (0.95, 0.5, 0.65), "brown": (0.35, 0.2, 0.1),
          "white": (0.95, 0.95, 0.95), "grey": (0.5, 0.5, 0.52), "gray": (0.5, 0.5, 0.52), "black": (0.03, 0.03, 0.03),
          "skin": (0.9, 0.65, 0.5), "wood": (0.45, 0.28, 0.12), "snow": (0.95, 0.97, 1.0), "grass": (0.2, 0.5, 0.15)}


def _rgb(c):
    """A colour name (red, brown, ...) or an (r, g, b) list."""
    if isinstance(c, str):
        if c.lower() not in COLORS:
            raise ValueError("unknown colour %s; use a name (%s) or [r, g, b] from 0 to 1" % (c, ", ".join(COLORS)))
        return COLORS[c.lower()]
    return tuple(float(v) for v in c)


def bbox(obj):
    """(low, high) corners of obj in the world."""
    bpy.context.view_layer.update()
    cs = [obj.matrix_world @ Vector(c) for c in obj.bound_box]
    return Vector([min(c[i] for c in cs) for i in range(3)]), Vector([max(c[i] for c in cs) for i in range(3)])


def add(shape, size=2, at=None, color=None, name=None, scale=None, rotation=None):
    """Add one shape and return it. shape: cube, sphere, plane, cylinder, cone, torus, monkey.
    size = width (a sphere's diameter); scale = (x, y, z) stretch, e.g. a flat slab: scale=(3, 2, 0.2).
    at = (x, y, z) place; rotation = (x, y, z) degrees. color = a name or (r, g, b)."""
    s = float(size)
    at = tuple(at) if at else (0, 0, 0)
    ops = {"cube": lambda: bpy.ops.mesh.primitive_cube_add(size=s, location=at),
           "sphere": lambda: bpy.ops.mesh.primitive_uv_sphere_add(radius=s / 2, segments=48, ring_count=24, location=at),
           "plane": lambda: bpy.ops.mesh.primitive_plane_add(size=s, location=at),
           "cylinder": lambda: bpy.ops.mesh.primitive_cylinder_add(radius=s / 2, depth=s, vertices=48, location=at),
           "cone": lambda: bpy.ops.mesh.primitive_cone_add(radius1=s / 2, depth=s, vertices=48, location=at),
           "torus": lambda: bpy.ops.mesh.primitive_torus_add(major_radius=s / 2, minor_radius=s / 8, location=at),
           "monkey": lambda: bpy.ops.mesh.primitive_monkey_add(size=s, location=at)}
    if shape not in ops:
        raise ValueError("unknown shape %s; use one of %s" % (shape, ", ".join(ops)))
    ops[shape]()
    obj = bpy.context.active_object
    if shape in ("sphere", "monkey", "torus", "cylinder"):
        obj.data.polygons.foreach_set("use_smooth", [True] * len(obj.data.polygons))
    if name:
        obj.name = name
    if scale is not None:
        obj.scale = (scale, scale, scale) if isinstance(scale, (int, float)) else tuple(scale)
    if rotation:
        obj.rotation_euler = tuple(math.radians(float(v)) for v in rotation)
    if color is not None:
        _PAINT(obj, _rgb(color))
    return obj


def ground_plane(size=400, color="grass", name="ground"):
    """A big flat floor at height 0; the camera ignores it when framing the scene."""
    g = add("plane", size, name=name, color=color)
    g["kk_ground"] = 1
    bpy.context.scene.world.node_tree.nodes["Background"].inputs[0].default_value = (0.3, 0.4, 0.55, 1)  # sky, so the floor has a horizon
    return g


def _move(obj, delta):
    obj.location = obj.location + Vector(delta)
    bpy.context.view_layer.update()


def put_on_ground(obj, z=0.0):
    """Lower or raise obj until its lowest point is at height z (the floor)."""
    lo, _ = bbox(obj)
    _move(obj, (0, 0, z - lo.z))
    return obj


def put_on(obj, base, offset=(0, 0), sink=0.0):
    """Rest obj on top of base, centred on it (offset = (dx, dy) from the centre). sink = fraction of obj's height that dips in."""
    blo, bhi = bbox(base)
    olo, ohi = bbox(obj)
    c = (blo + bhi) / 2
    _move(obj, (c.x + offset[0] - (olo.x + ohi.x) / 2, c.y + offset[1] - (olo.y + ohi.y) / 2, bhi.z - olo.z - sink * (ohi.z - olo.z)))
    return obj


def stack(objs, sink=0.0):
    """Put each object on top of the one before it: stack([a, b, c]) puts c on b on a."""
    for below, above in zip(objs, objs[1:]):
        put_on(above, below, sink=sink)
    return objs


def next_to(obj, other, side="right", gap=0.0):
    """Stand obj beside other (side: left, right, front, back) with gap between them, on the same floor level."""
    d = Vector(SIDES[side])
    blo, bhi = bbox(other)
    olo, ohi = bbox(obj)
    c = (blo + bhi) / 2
    ax = 0 if d.x else 1
    target = (bhi[ax] + gap + (ohi[ax] - olo[ax]) / 2) if d[ax] > 0 else (blo[ax] - gap - (ohi[ax] - olo[ax]) / 2)
    dv = [0, 0, blo.z - olo.z]
    dv[ax] = target - (olo[ax] + ohi[ax]) / 2
    other_ax = 1 - ax
    dv[other_ax] = c[other_ax] - (olo[other_ax] + ohi[other_ax]) / 2
    _move(obj, dv)
    return obj


def attach(obj, to, side="front", offset=(0, 0), sink=0.25):
    """Fix obj onto the surface of `to` on one side (front, back, left, right, top, bottom), sunk in by `sink` of its own size so it holds.
    offset = shift along the surface: front/back (x, z), left/right (y, z), top/bottom (x, y), from the centre of `to`."""
    d = Vector(SIDES[side])
    lo, hi = bbox(to)
    c = (lo + hi) / 2
    ax = [i for i in range(3) if d[i]][0]
    p = Vector(c)
    others = [i for i in range(3) if i != ax]
    p[others[0]] += offset[0]
    p[others[1]] += offset[1]
    face = hi[ax] if d[ax] > 0 else lo[ax]
    inv = to.matrix_world.inverted()
    hit, loc, _n, _i = to.ray_cast(inv @ (p + d * 200), (inv.to_3x3() @ (-d)).normalized())
    if hit:
        face = (to.matrix_world @ loc)[ax]
    olo, ohi = bbox(obj)
    ext = ohi[ax] - olo[ax]
    centre = face + d[ax] * (ext / 2 - sink * ext)
    dv = [0, 0, 0]
    dv[ax] = centre - (olo[ax] + ohi[ax]) / 2
    dv[others[0]] = p[others[0]] - (olo[others[0]] + ohi[others[0]]) / 2
    dv[others[1]] = p[others[1]] - (olo[others[1]] + ohi[others[1]]) / 2
    _move(obj, dv)
    return obj


def face_camera(obj):
    """Turn obj about the up axis so its front (a monkey's face, -Y) looks at the camera."""
    obj.rotation_euler.z += math.radians(CAM_AZ)
    bpy.context.view_layer.update()
    return obj


def _gap(a, b):
    (alo, ahi), (blo, bhi) = bbox(a), bbox(b)
    return Vector([max(blo[i] - ahi[i], alo[i] - bhi[i], 0.0) for i in range(3)]).length


def layout_report():
    """Facts measured from the scene itself: sizes, positions, what touches what, what floats."""
    objs = [o for o in bpy.data.objects if o.type == "MESH"]
    lines, warn = [], []
    for o in objs:
        lo, hi = bbox(o)
        dim = hi - lo
        tol = 0.04 * max(max(dim), 0.5)
        near = [p.name for p in objs if p is not o and _gap(o, p) <= tol]
        c = (lo + hi) / 2
        if o.get("kk_ground"):
            continue
        on_floor = [n for n in near if bpy.data.objects[n].get("kk_ground")]
        near = [n for n in near if n not in on_floor]
        lines.append("- %s: %.2f wide, %.2f deep, %.2f tall, centre (%.2f, %.2f, %.2f); %s" % (o.name, dim.x, dim.y, dim.z, c.x, c.y, c.z, "; ".join(
            (["stands on the ground"] if on_floor else []) + (["touches " + ", ".join(near)] if near else [])) or "touches nothing"))
        near = near + on_floor
        if not near and not o.get("kk_float"):
            warn.append("WARNING: %s touches nothing; it floats %.2f above the lowest object. Put it on or attach it to something." % (o.name, max(lo.z - min(bbox(p)[0].z for p in objs), 0)))
        if o.type == "MESH" and dim.z > 0.5 * max(dim.x, dim.y) and min(dim.x, dim.y) < 0.02 * max(dim) and not o.get("kk_ground"):
            warn.append("WARNING: %s is a flat sheet standing on its edge." % o.name)
    return "\n".join(lines + warn + NOTES + ([] if warn else ["All objects touch a neighbour; nothing floats."]))


# ---- BLD-03: scene graph. A plan (objects + how they relate) becomes placement, never loose coordinates.

def _template(name, p):
    """Ready-made object lists; the coordinates come from relations, so the parts always touch."""
    h = float(p.get("size", 3))
    if name == "snowman":
        a, b, c = 0.40 * h, 0.30 * h, 0.22 * h
        return [
            {"name": "bottom", "shape": "sphere", "size": a, "color": "snow"},
            {"name": "middle", "shape": "sphere", "size": b, "color": "snow", "on": "bottom", "sink": 0.12},
            {"name": "head", "shape": "sphere", "size": c, "color": "snow", "on": "middle", "sink": 0.12},
            {"name": "eye_left", "shape": "sphere", "size": c * 0.12, "color": "black", "attached_to": "head", "side": "front", "offset": [-c * 0.17, c * 0.12], "sink": 0.4},
            {"name": "eye_right", "shape": "sphere", "size": c * 0.12, "color": "black", "attached_to": "head", "side": "front", "offset": [c * 0.17, c * 0.12], "sink": 0.4},
            {"name": "nose", "shape": "cone", "size": c * 0.14, "scale": [1, 1, 3], "rotation": [90, 0, 0], "color": "orange", "attached_to": "head", "side": "front", "offset": [0, -c * 0.03], "sink": 0.2},
            {"name": "button_1", "shape": "sphere", "size": b * 0.09, "color": "black", "attached_to": "middle", "side": "front", "offset": [0, b * 0.15], "sink": 0.4},
            {"name": "button_2", "shape": "sphere", "size": b * 0.09, "color": "black", "attached_to": "middle", "side": "front", "offset": [0, -b * 0.05], "sink": 0.4},
            {"name": "button_3", "shape": "sphere", "size": b * 0.09, "color": "black", "attached_to": "middle", "side": "front", "offset": [0, -b * 0.25], "sink": 0.4},
            {"name": "arm_left", "shape": "cylinder", "size": b * 0.05, "scale": [1, 1, 12], "rotation": [0, 55, 0], "color": "brown", "attached_to": "middle", "side": "left", "offset": [0, b * 0.15], "sink": 0.1},
            {"name": "arm_right", "shape": "cylinder", "size": b * 0.05, "scale": [1, 1, 12], "rotation": [0, -55, 0], "color": "brown", "attached_to": "middle", "side": "right", "offset": [0, b * 0.15], "sink": 0.1},
            {"name": "hat", "shape": "cylinder", "size": c * 0.6, "scale": [1, 1, 0.9], "color": "black", "on": "head", "sink": 0.12},
        ]
    if name == "character":
        sk, sh, pa = p.get("skin_color", "skin"), p.get("shirt_color", "blue"), p.get("pants_color", "brown")
        return [
            {"name": "leg_left", "shape": "cylinder", "size": 0.12 * h, "scale": [1, 1, 3.4], "color": pa, "at": [-0.1 * h, 0]},
            {"name": "leg_right", "shape": "cylinder", "size": 0.12 * h, "scale": [1, 1, 3.4], "color": pa, "at": [0.1 * h, 0]},
            {"name": "torso", "shape": "cube", "size": 0.26 * h, "scale": [1, 0.6, 1.5], "color": sh, "on": "leg_left", "offset": [0.1 * h, 0], "sink": 0.0},
            {"name": "head", "shape": "sphere", "size": 0.24 * h, "color": sk, "on": "torso", "sink": 0.0},
            {"name": "arm_left", "shape": "cylinder", "size": 0.09 * h, "scale": [1, 1, 3.2], "color": sk, "attached_to": "torso", "side": "left", "offset": [0, 0.02 * h], "sink": 0.1},
            {"name": "arm_right", "shape": "cylinder", "size": 0.09 * h, "scale": [1, 1, 3.2], "color": sk, "attached_to": "torso", "side": "right", "offset": [0, 0.02 * h], "sink": 0.1},
            {"name": "eye_left", "shape": "sphere", "size": 0.03 * h, "color": "black", "attached_to": "head", "side": "front", "offset": [-0.04 * h, 0.02 * h], "sink": 0.4},
            {"name": "eye_right", "shape": "sphere", "size": 0.03 * h, "color": "black", "attached_to": "head", "side": "front", "offset": [0.04 * h, 0.02 * h], "sink": 0.4},
        ]
    if name == "table":
        w = h * 1.4
        legs = [{"name": "leg_%d" % i, "shape": "cylinder", "size": 0.08 * h, "scale": [1, 1, 8], "color": "wood", "attached_to": "top", "side": "bottom", "offset": [sx * (w / 2 - 0.1 * h), sy * (0.55 * h / 2 - 0.1 * h)], "sink": 0.0}
                for i, (sx, sy) in enumerate([(-1, -1), (1, -1), (-1, 1), (1, 1)])]
        return [{"name": "top", "shape": "cube", "size": 1, "scale": [w, 0.55 * h, 0.06 * h], "color": "wood", "at": [0, 0, 0.64 * h + 0.06 * h / 2]}] + legs
    raise ValueError("unknown template %s; use snowman, character or table" % name)


ALIASES = {"table": "top", "tabletop": "top", "table_top": "top", "floor": "ground", "plane": "ground", "base": "ground"}
NOTES = []


def _find(made, name):
    """A named object; the words models like (table, floor, plane) find the template's top or the ground."""
    return made.get(name) or made.get(ALIASES.get(str(name).lower(), "")) or None


def _place(obj, o, made):
    if o.get("location") and not o.get("at"):  # the old call shape
        o["at"] = o["location"]
    rel = [k for k in ("on", "attached_to", "next_to") if o.get(k)]
    if rel:
        target = _find(made, o[rel[0]])
        if target is None:
            raise ValueError("%s: %s is not defined before it; list %s earlier" % (o["name"], o[rel[0]], o[rel[0]]))
        off = o.get("offset") or [0, 0]
        if rel[0] == "on":
            put_on(obj, target, offset=off, sink=float(o.get("sink", 0)))
        elif rel[0] == "attached_to":
            attach(obj, target, o.get("side", "front"), off, float(o.get("sink", 0.25)))
        else:
            next_to(obj, target, o.get("side", "right"), float(o.get("gap", 0)))
    elif o.get("at") and len(o["at"]) >= 3:
        obj.location = tuple(float(v) for v in o["at"][:3])
        obj["kk_float"] = 1 if o.get("floating") else 0
        bpy.context.view_layer.update()
    else:
        if o.get("at"):
            _move(obj, (float(o["at"][0]), float(o["at"][1]), 0))
        put_on_ground(obj)


def build(spec):
    """Build a whole scene from a plan: {"template": "snowman"|"character"|"table", "size": 3, "objects": [...], "ground": true}.
    Each object: name, shape, size, scale, color, rotation, and one place: at [x, y] (stands on the floor) or at [x, y, z] (in the air),
    on <name>, attached_to <name> (+ side, offset), next_to <name> (+ side, gap). An object may only refer to one listed before it."""
    objs = list(spec.get("objects") or [])
    if spec.get("template"):
        tshapes = {t["shape"] for t in _template(spec["template"], spec)}
        odd = [o for o in objs if o.get("shape") not in tshapes | {"plane"} and not any(o.get(k) for k in ("on", "attached_to", "next_to"))]
        if odd and spec["template"] != "table":  # the chat wants something the template cannot make: build its own objects
            NOTES.append("ignored template %s: %s is not part of it" % (spec["template"], odd[0].get("name") or odd[0].get("shape")))
            spec = dict(spec, template=None)
        else:
            objs = _template(spec["template"], spec) + objs
    if spec.get("ground", True):
        g = spec.get("ground_color", "grass")
        ground_plane(color=g if g else "grass")
    made = {}
    if spec.get("ground", True):
        made["ground"] = bpy.data.objects["ground"]
    parts = {o["name"] for o in (_template(spec["template"], spec) if spec.get("template") else [])}
    for i, o in enumerate(objs):
        o = dict(o)
        o["name"] = str(o.get("name") or "%s_%d" % (o.get("shape", "object"), i + 1))
        if i >= len(parts) and spec.get("template") and o["name"] in parts:
            NOTES.append("ignored %s: the %s template already has a part with that name" % (o["name"], spec["template"]))
            continue
        if spec.get("template") and i >= len(parts) and not any(o.get(k) for k in ("on", "attached_to", "next_to")):
            if spec["template"] == "table":
                o["on"] = "top"  # loose things in a table scene go on the table
                o["offset"] = o.get("offset") or [(len(made) % 5 - 2) * 0.5 * float(spec.get("size", 3)) * 0.4, 0]
            else:
                NOTES.append("ignored %s: with the %s template, extra objects must be attached to it (on / attached_to / next_to)" % (o["name"], spec["template"]))
                continue
        if o.get("shape") == "plane" and o["name"].lower() in ("plane", "ground", "floor", "base") and not any(o.get(k) for k in ("on", "attached_to", "next_to")) and "ground" in made:
            if o.get("color") is not None:
                _PAINT(made["ground"], _rgb(o["color"]))
            NOTES.append("%s: the floor is always there; it is the ground" % o["name"])
            continue
        obj = add(o.get("shape", ""), o.get("size", 2), color=o.get("color"), name=o["name"], scale=o.get("scale"), rotation=o.get("rotation"))
        _place(obj, o, made)
        if o.get("face_camera"):
            face_camera(obj)
        made[o["name"]] = obj
    return made
