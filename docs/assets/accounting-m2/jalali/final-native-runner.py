"""Run an unchanged QA suite against the named immutable final executable."""
import hashlib
from pathlib import Path
import sys
suite = sys.argv[1]
assert suite in ('jalali-qa.py', 'accounting-qa.py')
source = Path('/workspace/sarv-quote/scripts') / suite
binary = Path('/workspace/sarv-evidence/bin/sarv-M2-layout-final')
assert hashlib.sha256(binary.read_bytes()).hexdigest() == '6c545a5642c53ac633a715f66f5f24cdfd107ccd58261724b688a89f1793a50d'
code = source.read_text()
original = "ROOT / 'target/release/sarv-quote'"
assert code.count(original) == 1
code = code.replace(original, "Path('/workspace/sarv-evidence/bin/sarv-M2-layout-final')")
sys.path.insert(0, str(source.parent))
exec(compile(code, str(source), 'exec'), {'__name__': '__main__', '__file__': str(source)})
