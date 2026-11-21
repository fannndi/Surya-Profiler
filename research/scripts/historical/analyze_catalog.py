import re, json, os
from pathlib import Path

ROOT = Path('/home/fannndi/Surya-Profiler')
ENTRIES = ROOT / 'MIUI-Fine-Tune/core/src/engine/catalog/entries.rs'
PROBE = ROOT / 'research/raw/catalog_probe.txt'
OUT = ROOT / 'research/analysis'
OUT.mkdir(parents=True, exist_ok=True)

src = ENTRIES.read_text()
pat = re.compile(r'\b(e|er|et|ei)!\s*\(\s*"([^"]+)"\s*,\s*"([^"]+)"\s*,\s*(\w+)\s*,\s*(\w+)', re.S)
rows = []
for m in pat.finditer(src):
    macro, key, path, tier, kind = m.groups()
    rows.append(dict(macro=macro, key=key, path=path, tier=tier, kind=kind))

probe = {}
for line in PROBE.read_text().splitlines()[1:]:
    parts = line.split('|')
    if len(parts) >= 6:
        probe[parts[0]] = dict(path=parts[1], exists=parts[2], perm=parts[3], owner=parts[4], value='|'.join(parts[5:]).strip())

with (OUT / 'catalog_miui14.md').open('w') as f:
    f.write('# Catalog probe — MIUI 14 V14.0.2.0.SJGIDXM / Android 12\n\n')
    f.write('Sumber: 95 entri katalog MiFineTune diprobe ulang di device (root).\n\n')
    f.write('| Key | Tier | Kind | Exists | Perm | Value | Path |\n')
    f.write('|---|---|---|---|---|---|---|\n')
    for r in rows:
        p = probe.get(r['key'], {})
        f.write('| `{}` | {} | {} | {} | {} | `{}` | `{}` |\n'.format(
            r['key'], r['tier'], r['kind'], p.get('exists','?'), p.get('perm','?'), p.get('value','?'), r['path']))

# Missing paths
missing = [r for r in rows if probe.get(r['key'], {}).get('exists') != 'yes']
with (OUT / 'catalog_missing.md').open('w') as f:
    f.write('# Catalog paths missing on MIUI 14\n\n')
    if missing:
        for r in missing:
            f.write(f'- `{r["key"]}` -> `{r["path"]}`\n')
    else:
        f.write('Tidak ada. Semua 95 path ada.\n')

# Perf XML overlap
import xml.etree.ElementTree as ET
perf_xml = ROOT / 'research/rom/perf/perf/commonresourceconfigs.xml'
text = perf_xml.read_text()
# strip comments to make XML parseable
text = re.sub(r'<!--.*?-->', '', text, flags=re.S)
root = ET.fromstring(text)
perf_nodes = []
major = None
for el in root.iter():
    if el.tag == 'Major':
        major = el.attrib.get('OpcodeValue')
    elif el.tag == 'Minor':
        node = el.attrib.get('Node','')
        if node.startswith('/'):
            perf_nodes.append((major, el.attrib.get('OpcodeValue'), node))
perf_paths = sorted(set(n for _,_,n in perf_nodes))
catalog_paths = {r['path']: r for r in rows}
overlap = [(m,mi,n) for m,mi,n in perf_nodes if n in catalog_paths]
with (OUT / 'perf_overlap.md').open('w') as f:
    f.write('# Perf HAL 2.2 resource overlap\n\n')
    f.write(f'Total resource path unik: {len(perf_paths)}. Overlap persis dengan katalog: {len(set(n for _,_,n in overlap))}.\n\n')
    f.write('| Major | Minor | Node | Catalog key |\n|---|---|---|---|\n')
    seen = set()
    for m,mi,n in overlap:
        if n in seen: continue
        seen.add(n)
        f.write(f'| {m} | {mi} | `{n}` | `{catalog_paths[n]["key"]}` |\n')
    f.write('\n## Semua path perf HAL\n\n')
    for p in perf_paths:
        f.write(f'- `{p}`\n')

# Profile key matrix
profiles = json.loads((ROOT / 'MIUI-Fine-Tune/core/profiles.json').read_text())['profiles']
keys = sorted({k for p in profiles for k in p['params']})
with (OUT / 'profile_matrix.md').open('w') as f:
    f.write('# Matriks nilai profil MiFineTune (referensi) vs default MIUI 14\n\n')
    f.write('| Key | Default MIUI 14 | ' + ' | '.join(p['id'] for p in profiles) + ' |\n')
    f.write('|---|---|' + '|'.join('---' for _ in profiles) + '|\n')
    for k in keys:
        default = probe.get(k, {}).get('value','-')
        vals = [p['params'].get(k,'') for p in profiles]
        f.write('| `{}` | `{}` | {} |\n'.format(k, default, ' | '.join(f'`{v}`' if v else '' for v in vals)))

print('wrote analysis docs')
