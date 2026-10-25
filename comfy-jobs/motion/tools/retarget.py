# STU-M3: put a generated motion (MoMask BVH, HumanML3D skeleton) onto a rigged 3D-01 character (pose editor skeleton).
# World-space retarget: each target bone turns by the same world rotation, relative to its rest, as its source bone;
# a yaw aligns the two rests (hip line) and the root moves by the hips' path scaled by leg length.
# Run: blender -b -P retarget.py -- RIG.glb MOTION.bvh CLIPNAME OUT.glb PREVIEW_PREFIX
import sys, math, bpy, mathutils
argv = sys.argv[sys.argv.index("--") + 1:]
rig_glb, bvh, clip, out, prev = argv
MAP = {"Hips": "pelvis", "Spine": "spine", "Spine2": "chest", "Neck": "neck", "Head": "head",
       "LeftArm": "l_shoulder", "LeftForeArm": "l_elbow", "LeftHand": "l_wrist",
       "RightArm": "r_shoulder", "RightForeArm": "r_elbow", "RightHand": "r_wrist",
       "LeftUpLeg": "l_hip", "LeftLeg": "l_knee", "LeftFoot": "l_ankle",
       "RightUpLeg": "r_hip", "RightLeg": "r_knee", "RightFoot": "r_ankle"}
bpy.ops.wm.read_factory_settings(use_empty=True)
bpy.ops.import_scene.gltf(filepath=rig_glb)
tgt = next(o for o in bpy.data.objects if o.type == "ARMATURE")
before = set(bpy.data.objects)
bpy.ops.import_anim.bvh(filepath=bvh, global_scale=1.0, update_scene_fps=True, update_scene_duration=True)
src = next(o for o in bpy.data.objects if o not in before and o.type == "ARMATURE")
scene = bpy.context.scene

def rest_world(arm, name):
    return (arm.matrix_world.to_3x3() @ arm.data.bones[name].matrix_local.to_3x3()).to_quaternion()

def hip_yaw(arm, l, r):
    a = arm.matrix_world @ arm.data.bones[l].head_local
    b = arm.matrix_world @ arm.data.bones[r].head_local
    return math.atan2(b.y - a.y, b.x - a.x)

G = mathutils.Quaternion((0, 0, 1), hip_yaw(tgt, "l_hip", "r_hip") - hip_yaw(src, "LeftUpLeg", "RightUpLeg"))
Gi = G.inverted()
src_leg = (src.data.bones["LeftUpLeg"].head_local - src.data.bones["LeftFoot"].head_local).length * src.matrix_world.to_scale()[0]
tgt_leg = (tgt.data.bones["l_hip"].head_local - tgt.data.bones["l_ankle"].head_local).length * tgt.matrix_world.to_scale()[0]
k = tgt_leg / src_leg

# Direction-based: the source armature is turned (G), scaled (k) and moved onto the target's pelvis; each target
# bone then points at its source bone's tail (Damped Track), the pelvis follows the hips, and the result is baked.
# (Matching world rotations relative to rest failed: the BVH rests in a T-pose, the rig with arms down, so arms folded.)
f0, f1 = int(src.animation_data.action.frame_range[0]), int(src.animation_data.action.frame_range[1])
scene.frame_set(f0)
src.rotation_mode = "QUATERNION"
src.rotation_quaternion = G @ src.matrix_world.to_quaternion()
src.scale = [v * k for v in src.scale]
bpy.context.view_layer.update()
src_hips_rest = src.matrix_world @ src.pose.bones["Hips"].head  # posed at the first frame (the BVH root carries the hip height)
pel_rest = tgt.matrix_world @ tgt.data.bones["pelvis"].head_local
src.location = src.location + (pel_rest - src_hips_rest)
bpy.context.view_layer.update()
scene.frame_set(f0)
for n in ("Hips", "Head", "LeftFoot", "LeftHand"):
    print("M3 dbg src", n, tuple(round(v, 2) for v in src.matrix_world @ src.pose.bones[n].head))
for n in ("pelvis", "head", "l_ankle", "l_wrist"):
    print("M3 dbg tgt", n, tuple(round(v, 2) for v in tgt.matrix_world @ tgt.pose.bones[n].head))
print("M3 dbg src scale", tuple(src.scale), "k", k, "src_leg", src_leg, "tgt_leg", tgt_leg)
ad = tgt.animation_data or tgt.animation_data_create()
for t in ad.nla_tracks:
    t.mute = True
act = bpy.data.actions.new(clip)
ad.action = act
# Each target bone is swung so the line to its child joint points where the source's does, parents first.
# (Damped Track failed: the glTF importer gives bones a Y axis that does not point at the child joint.)
CHAIN = [("spine", "chest", "Spine", "Spine2"), ("chest", "neck", "Spine2", "Neck"), ("neck", "head", "Neck", "Head"),
         ("l_shoulder", "l_elbow", "LeftArm", "LeftForeArm"), ("l_elbow", "l_wrist", "LeftForeArm", "LeftHand"),
         ("r_shoulder", "r_elbow", "RightArm", "RightForeArm"), ("r_elbow", "r_wrist", "RightForeArm", "RightHand"),
         ("l_hip", "l_knee", "LeftUpLeg", "LeftLeg"), ("l_knee", "l_ankle", "LeftLeg", "LeftFoot"),
         ("r_hip", "r_knee", "RightUpLeg", "RightLeg"), ("r_knee", "r_ankle", "RightLeg", "RightFoot")]
arm_q = tgt.matrix_world.to_quaternion()
for pb in tgt.pose.bones:
    pb.rotation_mode = "QUATERNION"
pel = tgt.pose.bones["pelvis"]
for f in range(f0, f1 + 1):
    scene.frame_set(f)
    for pb in tgt.pose.bones:
        pb.rotation_quaternion = (1, 0, 0, 0)
    pel.location = (0, 0, 0)
    bpy.context.view_layer.update()
    hips = src.matrix_world @ src.pose.bones["Hips"].head
    off = tgt.matrix_world.inverted() @ hips - tgt.matrix_world.inverted() @ pel_rest
    pel.location = pel.bone.matrix_local.to_3x3().inverted() @ off
    bpy.context.view_layer.update()
    pel.keyframe_insert("location", frame=f)
    pel.keyframe_insert("rotation_quaternion", frame=f)
    for t, tc, s0, s1 in CHAIN:
        pb = tgt.pose.bones[t]
        cur = (tgt.matrix_world @ tgt.pose.bones[tc].head) - (tgt.matrix_world @ pb.head)
        want = (src.matrix_world @ src.pose.bones[s1].head) - (src.matrix_world @ src.pose.bones[s0].head)
        if cur.length < 1e-6 or want.length < 1e-6:
            continue
        q_world = cur.normalized().rotation_difference(want.normalized())
        q_arm = arm_q.inverted() @ q_world @ arm_q
        m = pb.matrix.copy()
        r = (q_arm.to_matrix() @ m.to_3x3()).to_4x4()
        r.translation = m.translation
        pb.matrix = r
        bpy.context.view_layer.update()
        pb.keyframe_insert("rotation_quaternion", frame=f)
    for n in ("head", "l_wrist", "r_wrist", "l_ankle", "r_ankle"):
        tgt.pose.bones[n].keyframe_insert("rotation_quaternion", frame=f)
    if f == (f0 + f1) // 2:
        for a, b in (("RightHand", "r_wrist"), ("Head", "head"), ("RightForeArm", "r_elbow"), ("LeftFoot", "l_ankle")):
            print("M3 chk", f, a, tuple(round(v, 2) for v in src.matrix_world @ src.pose.bones[a].head), b, tuple(round(v, 2) for v in tgt.matrix_world @ tgt.pose.bones[b].head))
act = ad.action
act.name = clip
tr = ad.nla_tracks.new(); tr.name = clip
tr.strips.new(clip, f0, act)
ad.action = None
bpy.data.objects.remove(src, do_unlink=True)
for t in ad.nla_tracks:
    t.mute = False
bpy.ops.export_scene.gltf(filepath=out, export_format="GLB", export_animations=True, export_animation_mode="NLA_TRACKS", export_vertex_color="ACTIVE")
print("M3 wrote", out, "frames", f1 - f0 + 1, "scale", round(k, 3))

scene.render.engine = "BLENDER_WORKBENCH"; scene.display.shading.color_type = "VERTEX"
scene.render.resolution_x, scene.render.resolution_y = 384, 512
cam = bpy.data.objects.new("cam", bpy.data.cameras.new("cam")); scene.collection.objects.link(cam)
cam.data.type = "ORTHO"; cam.data.ortho_scale = 3.0; scene.camera = cam
for t in ad.nla_tracks:
    t.mute = t.name != clip
for i, frac in enumerate((0.15, 0.4, 0.65, 0.9)):
    scene.frame_set(int(f0 + (f1 - f0) * frac))
    cam.location = (3, -6, 1.0); cam.rotation_euler = (math.radians(90), 0, math.radians(27))
    scene.render.filepath = f"{prev}-{i}.png"
    bpy.ops.render.render(write_still=True)
print("M3 done")
