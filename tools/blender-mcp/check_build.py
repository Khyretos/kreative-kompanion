"""BLD-03: placement helpers in a real Blender. Run: blender -b --factory-startup --python-exit-code 1 --python tools/blender-mcp/check_build.py"""

import importlib.util
import os

import bpy

HERE = os.path.dirname(os.path.abspath(__file__)) if "__file__" in globals() else "/app"
_spec = importlib.util.spec_from_file_location("scene", os.path.join(HERE, "scene.py"))
scene = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(scene)
NOTES, bbox, build, layout_report, prepare = (
    scene.NOTES,
    scene.bbox,
    scene.build,
    scene.layout_report,
    scene.prepare,
)
_gap = scene._gap


def fresh():
    prepare()
    for m in list(bpy.data.meshes):
        bpy.data.meshes.remove(m)


def need(cond, msg):
    assert cond, msg


def gap(a, b):
    return _gap(bpy.data.objects[a], bpy.data.objects[b])


fresh()
build({"objects": [{"name": "monkey", "shape": "monkey", "size": 2}]})
lo, _ = bbox(bpy.data.objects["monkey"])
need(abs(lo.z) < 1e-3, "monkey rests on the floor, lowest point %s" % lo.z)
need(bpy.data.objects["ground"].dimensions.z < 1e-3, "ground is flat")

fresh()
build({"template": "snowman", "size": 3})
for a, b in [
    ("bottom", "middle"),
    ("middle", "head"),
    ("head", "hat"),
    ("head", "nose"),
    ("head", "eye_left"),
    ("middle", "button_1"),
    ("middle", "arm_left"),
    ("middle", "arm_right"),
]:
    need(gap(a, b) < 0.02, "%s and %s must touch (gap %.3f)" % (a, b, gap(a, b)))
need(
    abs(bbox(bpy.data.objects["bottom"])[0].z) < 1e-3,
    "bottom sphere stands on the floor",
)
need(
    bpy.data.objects["nose"].location.y < bpy.data.objects["head"].location.y,
    "nose points to the front (-Y)",
)
rep = layout_report()
need("nothing floats" in rep and "WARNING" not in rep, "snowman report: " + rep)

fresh()
build(
    {
        "objects": [
            {"name": "a", "shape": "cube", "size": 1},
            {"name": "b", "shape": "sphere", "size": 1, "at": [0, 0, 3]},
        ]
    }
)
rep = layout_report()
need("WARNING: b touches nothing" in rep, "a floating ball is reported: " + rep)
fresh()
build(
    {
        "objects": [
            {"name": "a", "shape": "cube", "size": 1},
            {
                "name": "b",
                "shape": "sphere",
                "size": 1,
                "at": [0, 0, 3],
                "floating": True,
            },
        ]
    }
)
need("WARNING" not in layout_report(), "floating on purpose is not a warning")

fresh()
build(
    {
        "template": "table",
        "size": 3,
        "objects": [{"name": "ball", "shape": "sphere", "size": 0.6, "on": "top"}],
    }
)
need(
    gap("ball", "top") < 0.02 and gap("leg_0", "top") < 0.02,
    "ball on the table, legs under it",
)
need(abs(bbox(bpy.data.objects["leg_0"])[0].z) < 1e-3, "table legs reach the floor")

fresh()
try:
    build({"objects": [{"name": "x", "shape": "sphere", "on": "nothing"}]})
    need(False, "an unknown base must raise")
except ValueError as e:
    need("not defined before it" in str(e), str(e))
fresh()
NOTES.clear()
build(
    {
        "template": "table",
        "size": 3,
        "objects": [
            {"name": "plane", "shape": "plane", "size": 5, "color": "grey"},
            {"name": "top", "shape": "cube"},
            {"name": "ball", "shape": "sphere", "size": 0.6, "on": "table"},
            {
                "name": "monkey",
                "shape": "monkey",
                "size": 1,
                "on": "plane",
                "at": [5, 5],
            },
        ],
    }
)
need(gap("ball", "top") < 0.02, "on 'table' finds the table top")
need(
    len([o for o in bpy.data.objects if o.name.startswith("top")]) == 1,
    "a duplicate template part is ignored",
)
need(
    sum(1 for o in bpy.data.objects if o.get("kk_ground")) == 1,
    "the model's plane is the ground, not a second plane",
)
need(any("ignored top" in n for n in NOTES), "ignored parts are reported")
fresh()
NOTES.clear()
build(
    {
        "template": "table",
        "size": 3,
        "objects": [{"name": "ball", "shape": "sphere", "size": 0.6, "at": [0, 1]}],
    }
)
need(gap("ball", "top") < 0.02, "a loose object in a table scene goes on the table")
fresh()
NOTES.clear()
build(
    {
        "template": "snowman",
        "objects": [{"name": "extra", "shape": "sphere", "at": [0, 0]}],
    }
)
need(
    "extra" not in bpy.data.objects and any("ignored extra" in n for n in NOTES),
    "loose extras on a snowman are ignored and reported",
)
fresh()
NOTES.clear()
build(
    {
        "template": "character",
        "objects": [{"name": "monkey", "shape": "monkey", "size": 2}],
    }
)
need(
    "monkey" in bpy.data.objects
    and "torso" not in bpy.data.objects
    and any("ignored template" in n for n in NOTES),
    "a monkey is not a character: the template is dropped",
)
print("CHECK OK")
