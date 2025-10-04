import json
import sys
from collections import OrderedDict

def convert_pins(pin_map):
    """
    Convert a dict of pins {id: pin_data} to a list of pin objects with 'id' field.
    """
    return [
        dict({"id": k}, **v) for k, v in pin_map.items()
    ]

def convert_node(node):
    """
    Convert 'inputs' and 'outputs' in a node from dict to list format if needed.
    """
    for key in ("inputs", "outputs"):
        if key in node and isinstance(node[key], dict):
            node[key] = convert_pins(node[key])

def main(path):
    with open(path, "r", encoding="utf-8") as f:
        data = json.load(f, object_pairs_hook=OrderedDict)
    if "nodes" in data:
        for node in data["nodes"].values():
            if isinstance(node, dict):
                convert_node(node)
    with open(path, "w", encoding="utf-8") as f:
        json.dump(data, f, indent=2)
    print(f"Converted {path} to new pin array format.")

if __name__ == "__main__":
    if len(sys.argv) != 2:
        print("Usage: python convert_bp_save.py <graph_save.json>")
        sys.exit(1)
    main(sys.argv[1])