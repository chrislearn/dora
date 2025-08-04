"""
This is just a simple demonstration of an MCP server.

The example returns some local information about the user's request, such as the tallest building, 
the happiest kindergarten, the best restaurant, etc.
"""

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
                case "best_restaurant":
                    node.send_output("reply", pa.array([f'{{"content":[{{"type": "text", "text": "{{\\"restaurant\\":\\"Taotie Refuses to Leave\\", \\"people\\": 888}}"}}]}}']), metadata=event["metadata"])
                case "tallest_building":
                    node.send_output("reply", pa.array([f'{{"content":[{{"type": "text", "text": "{{\\"building\\":\\"Zifeng Tower\\", \\"people\\": 1500}}"}}]}}']), metadata=event["metadata"])
                case "happiest_kindergarten":
                    node.send_output("reply", pa.array([f'{{"content":[{{"type": "text", "text": "{{\\"kindergarten\\":\\"Golden Sun Kindergarten\\", \\"children\\": 300}}"}}]}}']), metadata=event["metadata"])
                case _:
                    print(f"Unknown command: {name}")