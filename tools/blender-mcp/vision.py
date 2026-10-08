#!/usr/bin/env python3
"""BLD-03: describe a rendered picture using a vision model."""
import base64, json, os, urllib.request, urllib.error

def describe(png: bytes, names: list) -> str:
    url = os.environ.get("BLENDER_VISION_URL", "").strip()
    if not url:
        return ""
    
    timeout = 90
    try:
        t = os.environ.get("BLENDER_VISION_TIMEOUT")
        if t:
            timeout = int(t)
    except ValueError:
        pass
    
    model = os.environ.get("BLENDER_VISION_MODEL", "").strip()
    if not model:
        return ""
    
    key = os.environ.get("BLENDER_VISION_KEY", "")
    
    try:
        base_url = url.rstrip("/")
        endpoint = f"{base_url}/chat/completions"
        
        data = {
            "model": model,
            "max_tokens": 300,
            "reasoning_effort": "none",
            "messages": [
                {
                    "role": "user",
                    "content": [
                        {"type": "text", "text": f"Describe this rendered 3D scene. Say what objects you see, their colours and where they are in the picture (left, right, top, bottom). Do not judge whether things touch or float; that is measured separately. The scene was planned with these objects: {', '.join(names)}. Say which of them you cannot see. Describe only what you see; do not guess or invent anything. Answer in at most six short plain sentences, no lists, no markdown."},
                        {"type": "image_url", "image_url": {"url": f"data:image/png;base64,{base64.b64encode(png).decode('ascii')}"}}
                    ]
                }
            ]
        }
        
        req = urllib.request.Request(
            endpoint,
            data=json.dumps(data).encode("utf-8"),
            headers=dict({"Content-Type": "application/json"}, **({"Authorization": f"Bearer {key}"} if key else {})),
            method="POST"
        )
        
        with urllib.request.urlopen(req, timeout=timeout) as resp:
            if resp.status != 200:
                return ""
            body = json.loads(resp.read().decode("utf-8"))
            content = body.get("choices", [{}])[0].get("message", {}).get("content", "")
            return content.strip() if content else ""
            
    except Exception:
        return ""
