"""BLD-03: turn a failed Blender run into a short message the chat model can act on."""

import re

END = "Or call blender_scene with a plan; it needs no code."
NAMES = "add, color, ground_plane, put_on, put_on_ground, stack, attach, next_to, face_camera, build, bpy, math, Vector"


def explain_error(log, code):
    """The failing line of the chat's code, the error and a hint; the end of the log when no chat_code frame exists."""
    lines = (log or "").splitlines()
    frames = [
        i for i, ln in enumerate(lines) if re.search(r'File "chat_code", line \d+', ln)
    ]
    if not frames:
        tail = [ln for ln in lines if ln.strip()][-8:]
        return ("Blender said:\n" + "\n".join(tail) + "\n" if tail else "") + END
    i = frames[-1]
    n = int(re.search(r"line (\d+)", lines[i]).group(1))
    src = code.splitlines()[n - 1].strip() if 0 < n <= len(code.splitlines()) else ""
    err = next(
        (
            ln.strip()
            for ln in lines[i + 1 :]
            if re.match(r"\w*(Error|Exception)\b", ln.strip())
        ),
        "",
    )
    hint = (
        f"Names you can use: {NAMES}."
        if err.startswith("NameError")
        else "Use the helpers (add(shape, size, at, color, name) then put_on / attach) instead of raw bpy.ops calls."
    )
    return (
        f"Your code failed at line {n}: {src[:200]}\nError: {err[:200]}\n{hint}\n{END}"
    )
