import json
import sys
from pathlib import Path

payload = json.load(sys.stdin)
output_dir = Path(payload["output_dir"]) / "pre-hook.txt"
output_dir.write_text("pre hook executed via python runner\n", encoding="utf-8")
