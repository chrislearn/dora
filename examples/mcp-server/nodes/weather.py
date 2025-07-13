"""TODO: Add docstring."""

import pyarrow as pa
from dora import Node

node = Node()


for event in node:
    print("WWWWWWeather event:", event)
    node.send_output("text", pa.array(["text"]), metadata=event["metadata"])
