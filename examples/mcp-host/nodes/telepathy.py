"""
This is just a simple demonstration of an MCP server.

This MCP server has the ability of telepathy and can know who the current 
user's favorite star is and what their favorite movie is.
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
                case "telepathy":
                    node.send_output("reply", pa.array([f'{{"content":[{{"type": "text", "text": "{{\\"star\\":\\"Tom Hanks\\", \\"movie\\":\\"Forrest Gump\\"}}"}}]}}']), metadata=event["metadata"])
                case _:
                    print(f"Unknown command: {name}")