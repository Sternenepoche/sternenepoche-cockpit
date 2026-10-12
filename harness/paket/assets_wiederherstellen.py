"""Restore shared visual assets byte-for-byte from the pinned previous release."""
from pathlib import Path
import hashlib
import json
import urllib.request
import zipfile

root = Path(__file__).resolve().parents[2]
manifest = json.loads((root / 'asset-recovery.json').read_text())
missing = [a for a in manifest['assets'] if not (root / a['path']).is_file()]
if not missing:
    print('All shared visual assets present.')
    raise SystemExit(0)
cache = root / '.asset-cache'
cache.mkdir(exist_ok=True)
package = cache / 'cockpit-0.2.2.zip'
if not package.exists():
    with urllib.request.urlopen(manifest['base_url'], timeout=90) as source, package.open('wb') as target:
        while block := source.read(1024 * 1024):
            target.write(block)
assert hashlib.sha256(package.read_bytes()).hexdigest() == manifest['base_sha256'], 'Base release hash mismatch'
with zipfile.ZipFile(package) as archive:
    binary = archive.read('Sternenepoche-Harness-Cockpit/sternenepoche-cockpit.exe')
for asset in missing:
    target = (root / asset['path']).resolve()
    assert target.is_relative_to(root), 'Asset path outside checkout'
    data = binary[asset['offset']:asset['offset'] + asset['size']]
    assert hashlib.sha256(data).hexdigest() == asset['sha256'], asset['path']
    target.parent.mkdir(parents=True, exist_ok=True)
    target.write_bytes(data)
print(f'Restored {len(missing)} shared visual assets with exact SHA-256 checks.')
