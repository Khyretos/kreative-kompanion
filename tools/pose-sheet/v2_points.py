"""STU-SHEET3D: the 18 OpenPose points of the four v2 sheet figures (base_mesh.py's keypoints.json), computed without Blender."""

import math

# Joints in metres: +X is the figure's left, -Y its front, Z up; height 1.80.
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


def project(x, z, width, height):
    """Orthographic camera projection. Camera looks along +Y, centred at x=0, z=CAM_Z.
    Ortho scale spans the image width: s = width / (CELL * len(VIEWS)).
    Returns pixel coordinates [x_px, y_px]."""
    s = width / (CELL * len(VIEWS))
    return [round(width / 2 + x * s, 1), round(height / 2 - (z - CAM_Z) * s, 1)]


def rotate(p, deg):
    """Rotate point p about Z by deg degrees (Blender Matrix.Rotation style)."""
    c, s = math.cos(math.radians(deg)), math.sin(math.radians(deg))
    return (p[0] * c - p[1] * s, p[0] * s + p[1] * c, p[2])


def figure_points(i, width=1536, height=768):
    """Compute 18 OpenPose points for figure i.
    Figures are placed at x offset (i - (len(VIEWS)-1)/2) * CELL.
    Points are None if turned away from camera (dot(normal, (0,-1)) < -0.2)."""
    name, deg = VIEWS[i]
    x_offset = (i - (len(VIEWS) - 1) / 2) * CELL
    pts = []
    for k in ORDER:
        if k in FACE:
            # Head-to-point direction, rotated with the figure and flattened: turned away -> hidden
            head = J["head"]
            d = rotate((J[k][0] - head[0], J[k][1] - head[1], J[k][2] - head[2]), deg)
            length = math.hypot(d[0], d[1])
            if length and -d[1] / length < -0.2:  # dot((dx, dy) / length, (0, -1))
                pts.append(None)
                continue
        x, _, z = rotate(J[k], deg)
        pts.append(project(x + x_offset, z, width, height))
    return pts


def points(width=1536, height=768):
    """Return dict with width, height, order and list of figures with their points."""
    figures = []
    for i, (name, deg) in enumerate(VIEWS):
        figures.append({"view": name, "points": figure_points(i, width, height)})
    return {"width": width, "height": height, "order": ORDER, "figures": figures}
