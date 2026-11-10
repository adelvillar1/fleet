import json, sys
from pathlib import Path
sys.path.insert(0, str(Path("/Users/alejandrodelvillar/Projects/fleet/services/sys1/src")))
from sys1.api import classify
from sys1.types import Task, make_choice

SHAPES = ["state-sync", "change", "docs", "fail-open", "drops-the-row", "bound-exceeded",
          "claim-outran-evidence", "fixed-width-clip", "foreign-root", "incomplete",
          "resolved", "rendering", "deploy", "build", "deps"]
fixture = json.load(open("/tmp/c13-fixture.json"))
task = Task(
    id="shape_probe",
    heads=[make_choice("shape_probe", SHAPES, id="shape")],
    description=("Classify what the text is about, per fleet's fact-shape vocabulary: "
                 "state-sync = a durable fact about how things are wired or configured; "
                 "change = a code change or diff; docs = documentation; fail-open = a failure "
                 "that degrades named rather than refusing; build = a build or toolchain event; "
                 "the rest per their names."),
)
right = 0
rows = []
for fact in fixture:
    answer = classify(provider="glide", task=task, text=fact["text"]).head("shape", "glide")
    label = (answer or {}).get("label", "?")
    ok = label == fact["shape"]
    right += int(ok)
    rows.append((fact["file"][:40], fact["shape"], label))
print(f"head top-1 on the corpus's own labels: {right}/{len(fixture)} = {right/len(fixture)*100:.0f}%")
from collections import Counter
wrong = [(f, t, l) for f, t, l in rows if t != l]
print("misses:", len(wrong))
for f, t, l in wrong[:8]:
    print(f"  {f}: truth={t} head={l}")
