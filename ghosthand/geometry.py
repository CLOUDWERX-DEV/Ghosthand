"""Screen geometry: the output size and scale, and turning action words into numbers."""
import json
import os
import subprocess


def output_info(name=None):
    """An output as { name, width, height, scale }: size in screenshot pixels, scale as the compositor reports it (niri).

    GHOSTHAND_OUTPUT_SIZE="2560x1600" and GHOSTHAND_SCALE="1.25" override what niri says, for compositors without `niri msg`."""
    size, scale = os.environ.get("GHOSTHAND_OUTPUT_SIZE"), os.environ.get("GHOSTHAND_SCALE")
    if size and scale:
        w, h = size.lower().split("x")
        return {"name": os.environ.get("GHOSTHAND_OUTPUT", name or ""), "width": int(w), "height": int(h), "scale": float(scale)}
    outputs = json.loads(subprocess.run(["niri", "msg", "--json", "outputs"], capture_output=True, text=True, check=True).stdout)
    out = outputs.get(name) if name and name in outputs else next(iter(outputs.values()))
    mode = out["modes"][out["current_mode"]]
    return {"name": out.get("name", name or ""), "width": mode["width"], "height": mode["height"], "scale": (out.get("logical") or {}).get("scale") or 1.0}


def numbers(words):
    """The numbers among an action's words (floats, negatives and a leading + are fine); anything else is an error."""
    out = []
    for w in words:
        try:
            out.append(float(w))
        except ValueError:
            raise ValueError(f"not a number: {w!r}") from None
    return out
