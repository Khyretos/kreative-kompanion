# STU-P1: rig a 3D-01 character (.glb, static, vertex colours) with the pose editor's skeleton and give it four clips.
# Run: blender -b -P rig.py -- CHAR.glb OUT.glb PREVIEW_PREFIX clip-idle.glb clip-walk.glb clip-run.glb clip-attack.glb
import sys, bpy, mathutils
argv = sys.argv[sys.argv.index("--") + 1:]
char, out, prev, clips = argv[0], argv[1], argv[2], argv[3:]
bpy.ops.wm.read_factory_settings(use_empty=True)

def imported(path):
    before = set(bpy.data.objects)
    bpy.ops.import_scene.gltf(filepath=path)
    return [o for o in bpy.data.objects if o not in before]

arm = None
for path in clips:
    name = path.split("clip-")[-1].split(".")[0]
    objs = imported(path)
    a = next(o for o in objs if o.type == "ARMATURE")
    act = a.animation_data.action if a.animation_data else None
    if act is None:
        for t in (a.animation_data.nla_tracks if a.animation_data else []):
            for s in t.strips:
                act = s.action
    act.name = name
    act.use_fake_user = True
    if arm is None:
        arm = a
        arm.name = "KKRig"
        arm.animation_data.action = None
        keep = {arm}
    for o in objs:
        if o not in (arm,):
            bpy.data.objects.remove(o, do_unlink=True)
for act in list(bpy.data.actions):
    if act.name not in ("idle", "walk", "run", "attack"):
        bpy.data.actions.remove(act)
ad = arm.animation_data or arm.animation_data_create()
for act in [a for a in bpy.data.actions]:
    tr = ad.nla_tracks.new(); tr.name = act.name
    tr.strips.new(act.name, int(act.frame_range[0]), act.action if hasattr(act, "action") else act)
ad.action = None
for t in list(ad.nla_tracks):
    if t.name not in ("idle", "walk", "run", "attack"):
        ad.nla_tracks.remove(t)

# rest-pose height of the rig
bpy.context.view_layer.update()
zs = [(arm.matrix_world @ b.head_local).z for b in arm.data.bones] + [(arm.matrix_world @ b.tail_local).z for b in arm.data.bones]
rig_lo, rig_hi = min(zs), max(zs)

objs = imported(char)
meshes = [o for o in objs if o.type == "MESH"]
for o in objs:
    if o.type != "MESH":
        o.select_set(False)
bpy.ops.object.select_all(action="DESELECT")
for m in meshes:
    m.select_set(True)
bpy.context.view_layer.objects.active = meshes[0]
bpy.ops.object.parent_clear(type="CLEAR_KEEP_TRANSFORM")
bpy.ops.object.transform_apply(location=True, rotation=True, scale=True)
if len(meshes) > 1:
    bpy.ops.object.join()
body = bpy.context.view_layer.objects.active
body.name = "Character"
for o in objs:
    if o.name in bpy.data.objects and o.type != "MESH":
        bpy.data.objects.remove(o, do_unlink=True)
pts = [body.matrix_world @ v.co for v in body.data.vertices]
lo = mathutils.Vector((min(p.x for p in pts), min(p.y for p in pts), min(p.z for p in pts)))
hi = mathutils.Vector((max(p.x for p in pts), max(p.y for p in pts), max(p.z for p in pts)))
# the rig's top is the head bone tip; the mesh top includes hair: scale feet-to-top onto 1.04 x the rig span
s = (rig_hi - rig_lo) * 1.04 / (hi.z - lo.z)
body.scale = (s, s, s)
bpy.ops.object.transform_apply(scale=True)
pts = [v.co for v in body.data.vertices]
cx = (min(p.x for p in pts) + max(p.x for p in pts)) / 2
cy = (min(p.y for p in pts) + max(p.y for p in pts)) / 2
body.location = (-cx, -cy, rig_lo - min(p.z for p in pts))
bpy.ops.object.transform_apply(location=True)

# Weights: bone heat on a voxel-remeshed copy (TRELLIS meshes are not manifold, so heat on the mesh itself fails
# for 3 of 5 characters), then copied to the real mesh by nearest face.
for div in (150, 100, 70, 50):
    proxy = body.copy(); proxy.data = body.data.copy(); bpy.context.scene.collection.objects.link(proxy)
    rm = proxy.modifiers.new("remesh", "REMESH"); rm.mode = "VOXEL"; rm.voxel_size = (rig_hi - rig_lo) / div
    bpy.ops.object.select_all(action="DESELECT"); proxy.select_set(True); bpy.context.view_layer.objects.active = proxy
    bpy.ops.object.modifier_apply(modifier="remesh")
    bpy.ops.object.select_all(action="DESELECT")
    proxy.select_set(True); arm.select_set(True); bpy.context.view_layer.objects.active = arm
    bpy.ops.object.parent_set(type="ARMATURE_AUTO")
    pw = sum(1 for v in proxy.data.vertices if len(v.groups))
    print("RIG proxy voxel 1/%d: weighted %d of %d" % (div, pw, len(proxy.data.vertices)))
    if pw > 0.95 * len(proxy.data.vertices):
        break
    bpy.data.objects.remove(proxy, do_unlink=True)
for g in proxy.vertex_groups:
    body.vertex_groups.new(name=g.name)
dt = body.modifiers.new("weights", "DATA_TRANSFER"); dt.object = proxy
dt.use_vert_data = True; dt.data_types_verts = {"VGROUP_WEIGHTS"}; dt.vert_mapping = "POLYINTERP_NEAREST"
dt.layers_vgroup_select_src = "ALL"; dt.layers_vgroup_select_dst = "NAME"
bpy.ops.object.select_all(action="DESELECT"); body.select_set(True); bpy.context.view_layer.objects.active = body
bpy.ops.object.modifier_apply(modifier="weights")
bpy.data.objects.remove(proxy, do_unlink=True)
body.parent = arm
mod = body.modifiers.new("Armature", "ARMATURE"); mod.object = arm
weighted = sum(1 for v in body.data.vertices if len(v.groups))
print("RIG mesh weighted %d of %d vertices, %d bones, actions %s, scale %.3f" % (weighted, len(body.data.vertices), len(arm.data.bones), [a.name for a in bpy.data.actions], s))

bpy.ops.export_scene.gltf(filepath=out, export_format="GLB", export_animations=True, export_animation_mode="NLA_TRACKS", export_vertex_color="ACTIVE")
print("RIG wrote", out)

# previews: front camera, a frame from each clip
scene = bpy.context.scene
scene.render.engine = "BLENDER_WORKBENCH"
scene.display.shading.color_type = "VERTEX"
scene.render.resolution_x, scene.render.resolution_y = 384, 512
cam = bpy.data.objects.new("cam", bpy.data.cameras.new("cam")); scene.collection.objects.link(cam)
cam.data.type = "ORTHO"; cam.data.ortho_scale = (rig_hi - rig_lo) * 1.3
scene.camera = cam
for name, frac in (("idle", 0.5), ("walk", 0.25), ("run", 0.25), ("attack", 0.55)):
    act = bpy.data.actions[name]
    for t in ad.nla_tracks:
        t.mute = t.name != name
    f0, f1 = act.frame_range
    scene.frame_set(int(f0 + (f1 - f0) * frac))
    for side, loc, rot in (("front", (0, -4, 0), (90, 0, 0)), ("side", (4, 0, 0), (90, 0, 90))):
        cam.location = (loc[0], loc[1], (rig_lo + rig_hi) / 2); cam.rotation_euler = [r * 3.14159265 / 180 for r in rot]
        scene.render.filepath = f"{prev}-{name}-{side}.png"
        bpy.ops.render.render(write_still=True)
print("RIG done")
