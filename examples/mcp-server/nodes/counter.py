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
            match name:
                case "counter_increment":
                    node.send_output("reply", pa.array(['{"content":[{"type": "text", "text": "1"}]}']), metadata=event["metadata"])
                case "counter_decrement":
                    node.send_output("reply", pa.array(['{"content":[{"type": "text", "text": "-1"}]}']), metadata=event["metadata"])
                case "counter_get_value":
                    node.send_output("reply", pa.array(['{"content":[{"type": "text", "text": "0"}]}']), metadata=event["metadata"])
                case _:
                    print(f"Unknown command: {name}")