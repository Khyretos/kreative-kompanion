"""BLD-01: runs inside Blender. server.py prepends this text to every script."""

import bpy
import math
from mathutils import Vector


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
    for o in bpy.data.objects:
        if o.type in valid_types and o.bound_box:
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
            cam_obj.location = center + Vector((1, -1, 0.8)).normalized() * max(size * 2.2, 3.0)
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
        scene.cycles.samples = 32
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
    bpy.ops.render.render(write_still=True)
    print("rendered", out)
