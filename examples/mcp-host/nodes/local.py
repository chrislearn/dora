"""TODO: Add docstring."""

import pyarrow as pa
from dora import Node
import json


node = Node()

for event in node:
    if event["type"] == "INPUT":
        if 'metadata' in event:
            data = json.loads(event["value"][0].as_py())
            name = data.get("name", "")
            location = data.get("arguments", {}).get("location", "")
            match name:
                case "tallest_building":
                    node.send_output("reply", pa.array([f'{{"content":[{{"type": "text", "text": "tallest building in {location} is aaaaaa"}}]}}']), metadata=event["metadata"])
                case "delicious_food":
                    node.send_output("reply", pa.array([f'{{"content":[{{"type": "text", "text": "delicious food in {location} is bbbbbb"}}]}}']), metadata=event["metadata"])
                case _:
                    print(f"Unknown command: {name}")