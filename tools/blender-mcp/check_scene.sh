#!/bin/sh
# BLD-01: runs scene.py in a real headless Blender (default: soucouyant's flatpak over ssh).
# Usage: sh tools/blender-mcp/check_scene.sh [ssh-host]
HOST=${1:-khyretos@192.168.178.80}
F=tools/blender-mcp/scene.py
test -s $F || {
    echo "error: $F: not written"
    exit 1
}
python3 -c "import ast,sys; ast.parse(open('$F').read())" 2>&1 | sed "s|^|error: $F: |" || exit 1
T=$(mktemp)
{
    cat $F
    printf '\nprepare()\nimport bpy, os\nassert len(bpy.data.objects) == 0, "prepare left objects"\nbg = bpy.context.scene.world.node_tree.nodes["Background"].inputs[0].default_value\nassert abs(bg[0] - 0.05) < 0.001 and abs(bg[2] - 0.06) < 0.001, "prepare: background colour not set on the existing world"\nbpy.ops.mesh.primitive_monkey_add(location=(2, 0, 0))\nc, s = _bounds()\nassert abs(c.x - 2) < 0.05 and abs(c.y) < 0.05, "_bounds: center %%s, want (2, 0, z): lo/hi must be per-axis min/max" %% (tuple(c),)\nassert 2.5 < s < 3.0, "_bounds: size %%s, want the largest axis extent (about 2.7)" %% s\nm = bpy.context.active_object\ncolor(m, (1, 0, 0))\nmat = m.data.materials[0]\nbc = mat.node_tree.nodes["Principled BSDF"].inputs["Base Color"].default_value\nassert tuple(round(x, 3) for x in bc) == (1, 0, 0, 1), "color: Base Color %%s" %% (tuple(bc),)\nassert tuple(round(x, 3) for x in mat.diffuse_color) == (1, 0, 0, 1), "color: diffuse_color"\ncolor(m, (0, 0, 1, 0.5))\nassert len(m.data.materials) == 1 and m.active_material.node_tree.nodes["Principled BSDF"].inputs["Base Color"].default_value[2] == 1, "color: a second call replaces the material"\nfinish("/tmp/kk-bld-chk.png", 64, 48, "EEVEE")\nsc = bpy.context.scene\nassert sc.camera is not None, "no camera"\nassert any(o.type == "LIGHT" for o in bpy.data.objects), "no light"\nassert sc.render.resolution_x == 64 and sc.render.resolution_y == 48, "size"\nassert os.path.getsize("/tmp/kk-bld-chk.png") > 100, "no png"\nprint("CHECK OK")\n'
} >"$T"
scp -q "$T" "$HOST":/tmp/kk-bld-chk.py && rm -f "$T"
OUT=$(ssh "$HOST" "bash -c 'rm -f /tmp/kk-bld-chk.png; timeout 120 flatpak run --filesystem=/tmp org.blender.Blender -b --factory-startup --python-exit-code 1 --python /tmp/kk-bld-chk.py 2>&1'")
echo "$OUT" | grep -q "CHECK OK" && {
    echo ok
    exit 0
}
echo "$OUT" | grep -E "Error|error|assert|Traceback|line [0-9]" | head -15 | sed "s|^|error: $F: |"
exit 1
