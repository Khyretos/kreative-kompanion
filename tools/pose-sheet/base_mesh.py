"""STU-C1: the base mesh for the four-view character sheet.

Run inside Blender (4.x):  blender -b -P base_mesh.py -- <out_dir> [width height]
Builds a plain mannequin (about 8 heads tall, arms in a slight A-pose), puts four copies in a
row facing front, left, back and right, and renders them with an orthographic camera:
  <out_dir>/depth.png      near = white, background black (ControlNet depth)
  <out_dir>/keypoints.json the 18 OpenPose (COCO) points of every figure in pixels, null when hidden
draw_pose.py turns keypoints.json into the OpenPose image. Each figure owns a quarter of the
width, so the views cannot swap places.
"""

import json
import math
import os
import sys

import bpy
from bpy_extras.object_utils import world_to_camera_view
from mathutils import Matrix, Vector

args = sys.argv[sys.argv.index("--") + 1 :] if "--" in sys.argv else []
OUT = args[0] if args else "."
W, H = (int(args[1]), int(args[2])) if len(args) > 2 else (1536, 768)

# Body-local joints in metres: +X is the figure's left, -Y its front, Z up; height 1.80.
J = {
    "head": (0, 0, 1.68),
    "nose": (0, -0.105, 1.66),
    "r_eye": (-0.035, -0.092, 1.70),
    "l_eye": (0.035, -0.092, 1.70),
    "r_ear": (-0.088, 0, 1.68),
    "l_ear": (0.088, 0, 1.68),
    "neck": (0, 0, 1.49),
    "r_shoulder": (-0.19, 0, 1.46),
    "l_shoulder": (0.19, 0, 1.46),
    "r_elbow": (-0.29, 0, 1.18),
    "l_elbow": (0.29, 0, 1.18),
    "r_wrist": (-0.37, -0.03, 0.93),
    "l_wrist": (0.37, -0.03, 0.93),
    "r_hip": (-0.10, 0, 0.95),
    "l_hip": (0.10, 0, 0.95),
    "r_knee": (-0.11, -0.01, 0.52),
    "l_knee": (0.11, -0.01, 0.52),
    "r_ankle": (-0.11, 0.01, 0.09),
    "l_ankle": (0.11, 0.01, 0.09),
}
# OpenPose body order (COCO 18).
ORDER = [
    "nose",
    "neck",
    "r_shoulder",
    "r_elbow",
    "r_wrist",
    "l_shoulder",
    "l_elbow",
    "l_wrist",
    "r_hip",
    "r_knee",
    "r_ankle",
    "l_hip",
    "l_knee",
    "l_ankle",
    "r_eye",
    "l_eye",
    "r_ear",
    "l_ear",
]
FACE = {"nose", "r_eye", "l_eye", "r_ear", "l_ear"}
# Front, profile facing left, back, profile facing right (rotation about Z, degrees).
VIEWS = [("front", 0), ("left", -90), ("back", 180), ("right", 90)]
CELL = 1.15  # metres of sheet per figure
CAM_Z = 0.92
CAM_Y = -10.0


def capsule(a, b, r, parent):
    a, b = Vector(a), Vector(b)
    d = b - a
    bpy.ops.mesh.primitive_cylinder_add(
        radius=r, depth=d.length, location=(a + b) / 2, vertices=24
    )
    cyl = bpy.context.object
    cyl.rotation_mode = "QUATERNION"
    cyl.rotation_quaternion = d.to_track_quat("Z", "Y")
    parts = [cyl]
    for p in (a, b):
        bpy.ops.mesh.primitive_uv_sphere_add(
            radius=r, location=p, segments=24, ring_count=12
        )
        parts.append(bpy.context.object)
    for o in parts:
        o.parent = parent


def blob(loc, size, parent):
    bpy.ops.mesh.primitive_uv_sphere_add(
        radius=1, location=loc, segments=32, ring_count=16
    )
    o = bpy.context.object
    o.scale = size
    o.parent = parent


def figure(name):
    root = bpy.data.objects.new(name, None)
    bpy.context.collection.objects.link(root)
    blob(J["head"], (0.088, 0.10, 0.118), root)
    blob(J["nose"], (0.016, 0.02, 0.02), root)
    blob((0, -0.02, 1.33), (0.165, 0.105, 0.17), root)  # chest
    blob((0, -0.005, 1.13), (0.135, 0.09, 0.13), root)  # waist
    blob((0, 0, 0.98), (0.155, 0.10, 0.11), root)  # pelvis
    capsule((0, 0, 1.47), (0, 0, 1.58), 0.048, root)  # neck
    for s in ("r", "l"):
        capsule(J[f"{s}_shoulder"], J[f"{s}_elbow"], 0.046, root)
        capsule(J[f"{s}_elbow"], J[f"{s}_wrist"], 0.038, root)
        w = Vector(J[f"{s}_wrist"])
        blob(
            w + Vector((0.012 if s == "l" else -0.012, -0.01, -0.07)),
            (0.03, 0.045, 0.07),
            root,
        )
        capsule(J[f"{s}_hip"], J[f"{s}_knee"], 0.075, root)
        capsule(J[f"{s}_knee"], J[f"{s}_ankle"], 0.055, root)
        a = Vector(J[f"{s}_ankle"])
        blob(
            a + Vector((0, -0.06, -0.04)), (0.045, 0.12, 0.04), root
        )  # foot points forward
    return root


def depth_material():
    """Emission = camera depth, the nearest point white and the far side dark grey."""
    m = bpy.data.materials.new("depth")
    m.use_nodes = True
    nt = m.node_tree
    nt.nodes.clear()
    cam = nt.nodes.new("ShaderNodeCameraData")
    rng = nt.nodes.new("ShaderNodeMapRange")
    rng.inputs["From Min"].default_value = -CAM_Y - 0.25
    rng.inputs["From Max"].default_value = -CAM_Y + 0.25
    rng.inputs["To Min"].default_value = 1.0
    rng.inputs["To Max"].default_value = 0.15
    em = nt.nodes.new("ShaderNodeEmission")
    out = nt.nodes.new("ShaderNodeOutputMaterial")
    nt.links.new(cam.outputs["View Z Depth"], rng.inputs["Value"])
    nt.links.new(rng.outputs["Result"], em.inputs["Color"])
    nt.links.new(em.outputs["Emission"], out.inputs["Surface"])
    return m


def main():
    bpy.ops.wm.read_factory_settings(use_empty=True)
    scene = bpy.context.scene
    mat = depth_material()
    roots = []
    for i, (name, deg) in enumerate(VIEWS):
        root = figure(name)
        x = (i - (len(VIEWS) - 1) / 2) * CELL
        root.matrix_world = Matrix.Translation((x, 0, 0)) @ Matrix.Rotation(
            math.radians(deg), 4, "Z"
        )
        roots.append((name, deg, root))
    bpy.context.view_layer.update()
    for o in scene.objects:
        if o.type == "MESH":
            o.data.materials.append(mat)
            for p in o.data.polygons:
                p.use_smooth = True

    cam_data = bpy.data.cameras.new("cam")
    cam_data.type = "ORTHO"
    cam_data.ortho_scale = CELL * len(VIEWS)
    cam = bpy.data.objects.new("cam", cam_data)
    scene.collection.objects.link(cam)
    cam.location = (0, CAM_Y, CAM_Z)
    cam.rotation_euler = (math.radians(90), 0, 0)
    scene.camera = cam

    scene.render.engine = "CYCLES"
    scene.cycles.samples = 4
    scene.cycles.use_denoising = False
    scene.render.resolution_x, scene.render.resolution_y = W, H
    scene.render.resolution_percentage = 100
    scene.view_settings.view_transform = "Standard"
    world = bpy.data.worlds.new("w")
    world.color = (0, 0, 0)
    scene.world = world
    scene.render.image_settings.color_mode = "RGB"
    os.makedirs(OUT, exist_ok=True)
    scene.render.filepath = os.path.join(OUT, "depth.png")
    bpy.ops.render.render(write_still=True)

    toward_cam = Vector((0, -1, 0))
    figures = []
    for name, deg, root in roots:
        rot = Matrix.Rotation(math.radians(deg), 3, "Z")
        pts = []
        for k in ORDER:
            if k in FACE:
                d = rot @ (Vector(J[k]) - Vector(J["head"]))
                d.z = 0
                if d.length and d.normalized().dot(toward_cam) < -0.2:
                    pts.append(None)  # turned away from the camera
                    continue
            v = world_to_camera_view(scene, cam, root.matrix_world @ Vector(J[k]))
            pts.append([round(v.x * W, 1), round((1 - v.y) * H, 1)])
        figures.append({"view": name, "points": pts})
    with open(os.path.join(OUT, "keypoints.json"), "w") as f:
        json.dump(
            {"width": W, "height": H, "order": ORDER, "figures": figures}, f, indent=1
        )


main()
