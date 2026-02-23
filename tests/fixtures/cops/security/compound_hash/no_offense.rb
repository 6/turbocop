obj.hash
"string".hash
x.hash
42.hash
{}.hash

# Multi-element array .hash is the recommended pattern
[a, b].hash
[x, y, z].hash
[1, 2].hash
[@foo, @bar].hash

# Inside a def hash method — this is the correct implementation pattern
def hash
  [control_node.object_id, control_node.object_id].hash
end
