"""Attribute an arm64 Mach-O file; physical bytes, not its virtual address space."""
import argparse
from collections import defaultdict
import hashlib
import json
from pathlib import Path
import re
import struct

p = argparse.ArgumentParser(description=__doc__)
p.add_argument("--binary", type=Path, required=True)
p.add_argument("--map", type=Path)
p.add_argument("--trident", type=Path, required=True)
p.add_argument("--output", type=Path, required=True)
a = p.parse_args()
b = a.binary.read_bytes()
assert struct.unpack_from("<I", b)[0] == 0xFEEDFACF
ncmds = struct.unpack_from("<I", b, 16)[0]
segments, sections, symtab = [], [], {}
offset = 32
name = lambda raw: raw.split(b"\0")[0].decode()
for _ in range(ncmds):
    cmd, length = struct.unpack_from("<II", b, offset)
    if cmd == 0x19:
        v = struct.unpack_from("<II16sQQQQiiII", b, offset)
        segments.append(dict(name=name(v[2]), file_offset=v[5], file_bytes=v[6]))
        for i in range(v[9]):
            s = struct.unpack_from("<16s16sQQIIIIIIII", b, offset + 72 + 80*i)
            sections.append(dict(segment=name(s[1]), name=name(s[0]), address=s[2],
                                 bytes=s[3], offset=s[4], zero_fill=s[8] & 255 in (1,12,18)))
    elif cmd == 2:
        _, _, symoff, count, stroff, strbytes = struct.unpack_from("<IIIIII", b, offset)
        symtab = dict(symbols=count, symbol_table_bytes=count*16, string_table_bytes=strbytes)
    offset += length
assert sum(s["file_bytes"] for s in segments) == len(b)
report = dict(binary=str(a.binary), bytes=len(b), sha256=hashlib.sha256(b).hexdigest(),
              segments=segments, sections=sections, symbol_metadata=symtab)
if a.map:
    objects, totals, symbols, mode = {}, defaultdict(int), [], ""
    text = next(s for s in sections if s["segment"] == "__TEXT" and s["name"] == "__text")
    for line in a.map.read_text(errors="replace").splitlines():
        if line.startswith("# "):
            if line.endswith(":"):
                mode = line
            continue
        if mode == "# Object files:":
            m = re.match(r"\[\s*(\d+)\] (.*)", line)
            if m:
                objects[int(m[1])] = m[2]
        elif mode == "# Symbols:":
            m = re.match(r"(0x[0-9A-Fa-f]+)\s+(0x[0-9A-Fa-f]+)\s+\[\s*(\d+)\]\s+(.*)", line)
            if not m:
                continue
            address, size, obj = (int(m[1],16), int(m[2],16), int(m[3]))
            if text["address"] <= address < text["address"] + text["bytes"]:
                source = objects[obj]
                crate = re.search(r"/lib([a-zA-Z0-9_]+)-[^/]+\.(?:rlib|a)\(", source)
                owner = crate[1] if crate else "joy-cli/" + ("generated" if obj else "linker")
                totals[owner] += size
                symbols.append(dict(bytes=size, object_owner=owner, symbol=m[4]))
    report["code_by_object_owner"] = dict(sorted(totals.items(), key=lambda x: -x[1]))
    report["code_attribution_note"] = "Live linker-map symbol sizes grouped by containing object archive; includes instantiated generics, not semantic ownership. Padding remains unattributed."
    report["code_unattributed_bytes"] = text["bytes"] - sum(totals.values())
    report["largest_code_symbols"] = sorted(symbols, key=lambda s: -s["bytes"])[:25]
resources, intervals = [], []
for f in sorted((a.trident/"lib").rglob("*")):
    if not f.is_file() or f.suffix not in (".tri", ".toml"):
        continue
    relative = f.relative_to(a.trident).as_posix()
    data = f.read_bytes()
    start = b.find(data) if data else -1
    resources.append(dict(path=relative, bytes=len(data), present=start >= 0))
    if start >= 0:
        intervals.append((start, start+len(data)))
end, total = 0, 0
for start, stop in sorted(intervals):
    total += max(0, stop - max(start, end))
    end = max(end, stop)
report["embedded_resources"] = dict(unique_physical_bytes=total, files=resources,
    note="Union of first exact content matches; subset of constants, never added again to file total.")
a.output.write_text(json.dumps(report, indent=2) + "\n")
print(json.dumps({k: report[k] for k in ("bytes", "sha256", "segments", "symbol_metadata")}, indent=2))
if a.map:
    print(json.dumps(report["code_by_object_owner"], indent=2))
print(json.dumps(dict(embedded_unique_bytes=total)))
