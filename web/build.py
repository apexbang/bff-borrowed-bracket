"""Inline sim.js and the default seed into page.html -> soup.html.
Also writes preview.html with a doctype wrapper for local screenshots."""
import sys
seed = sys.argv[1] if len(sys.argv) > 1 else "1"
page = open("page.html").read()
sim = open("sim.js").read()
assert "</script" not in sim
import base64
birth = base64.b64encode(open("crop_7904.bin", "rb").read()).decode()
import json
meta = json.load(open("births/meta.json"))
births = []
for name, m in meta.items():
    rec = open(f"births/{name}", "rb").read()
    assert len(rec) == 129, name
    births.append({**m, "tape": list(rec[:128]), "child": rec[128]})
page = page.replace("/*__BIRTHS__*/", json.dumps(births, separators=(",", ":")))
out = page.replace("/*__SIM__*/", sim).replace("/*__SEED__*/", seed).replace("/*__BIRTH__*/", birth)
open("soup.html", "w").write(out)
open("preview.html", "w").write(
    '<!doctype html><html><head><meta charset="utf-8">'
    '<meta name="viewport" content="width=device-width,initial-scale=1,viewport-fit=cover">'
    '<style>body{margin:0}[hidden]{display:none!important}</style></head><body>' + out + "</body></html>")
print("built with seed", seed)
