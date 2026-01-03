import sys
import os
import site

# 1. Get the directory of the executable
if getattr(sys, 'frozen', False):
    base_dir = os.path.dirname(os.path.abspath(sys.executable))
else:
    base_dir = os.path.dirname(os.path.abspath(__file__))

# 2. Define the path to our harvested libs
# We will tell Rust to put them in a folder named 'lib' next to the binary
libs_dir = os.path.join(base_dir, "lib")

# 3. Inject into sys.path
sys.path.insert(0, libs_dir)
site.addsitedir(libs_dir)

# 4. Import the actual logic
# We assume 'ppocr.py' is also next to the binary
sys.path.insert(0, base_dir)

try:
    import ppocr
    ppocr.main()
except ImportError as e:
    import json
    print(json.dumps({"error": f"Bootloader Error: {e}", "path": sys.path}))
