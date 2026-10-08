from pathlib import Path
import sys
script=Path('/workspace/sarv-quote/scripts/product-qa.py')
sys.path.insert(0,str(script.parent))
source=script.read_text()
old="binary = ROOT / 'target/release/sarv-quote'"
assert source.count(old)==1
source=source.replace(old,"binary = Path(os.environ['SARV_QUOTE_DIAG_BINARY'])")
source=source.replace('            x.key(name, *mods)', '            print("M4 DIAGNOSTIC KEY", time.time(), name, mods, flush=True)\n            x.key(name, *mods)')
source=source.replace('            row = native.wait_for(selected_action', '            print("M4 DIAGNOSTIC ACTION", time.time(), name, flush=True)\n            row = native.wait_for(selected_action')
exec(compile(source,str(script),'exec'),{'__name__':'__main__','__file__':str(script)})
