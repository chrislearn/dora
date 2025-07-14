"""TODO: Add docstring."""

import pyarrow as pa
from dora import Node

node = Node()


for event in node:
    print("WWWWWcounter event:", event)
    if 'metadata' in event:
        node.send_output("reply", pa.array(["text"]), metadata=event["metadata"])
