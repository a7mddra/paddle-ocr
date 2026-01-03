#!/usr/bin/env python3
import sys
import os
import json
import logging
import numpy as np
from paddleocr import PaddleOCR

logging.getLogger("ppocr").setLevel(logging.ERROR)
os.environ["DISABLE_MODEL_SOURCE_CHECK"] = "True"

class NumpyEncoder(json.JSONEncoder):
    def default(self, obj):
        if isinstance(obj, np.integer):
            return int(obj)
        elif isinstance(obj, np.floating):
            return float(obj)
        elif isinstance(obj, np.ndarray):
            return obj.tolist()
        return super(NumpyEncoder, self).default(obj)

def main():
    if len(sys.argv) < 2:
        print(json.dumps({"error": "No image path provided"}))
        sys.exit(1)

    image_path = sys.argv[1]
    
    if not os.path.exists(image_path):
        print(json.dumps({"error": f"Image not found at {image_path}"}))
        sys.exit(1)

    try:
        ocr = PaddleOCR(use_angle_cls=True, lang='en', show_log=False)
    except Exception as e:
        print(json.dumps({"error": f"Failed to initialize PaddleOCR: {str(e)}"}))
        sys.exit(1)

    try:
        result = ocr.ocr(image_path, cls=True)
    except Exception as e:
        print(json.dumps({"error": str(e)}))
        sys.exit(1)

    if result is None or len(result) == 0 or result[0] is None:
        print(json.dumps([]))
        return

    output_data = []
    
    for line in result[0]:
        if len(line) >= 2:
            box = line[0]
            text = line[1][0]
            output_data.append({
                "text": text,
                "box": box
            })

    print(json.dumps(output_data, cls=NumpyEncoder))

if __name__ == "__main__":
    main()