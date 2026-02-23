# Monuple: single value wrapped in array is redundant
[single_value].hash
               ^^^^ Security/CompoundHash: Delegate hash directly without wrapping in an array when only using a single value.
[@foo].hash
       ^^^^ Security/CompoundHash: Delegate hash directly without wrapping in an array when only using a single value.
[x].hash
    ^^^^ Security/CompoundHash: Delegate hash directly without wrapping in an array when only using a single value.

# Redundant: calling .hash on elements of hashed array
[@foo.hash, @bar.hash].hash
                       ^^^^ Security/CompoundHash: Calling `.hash` on elements of a hashed array is redundant.
[a.hash, b.hash, c.hash].hash
                         ^^^^ Security/CompoundHash: Calling `.hash` on elements of a hashed array is redundant.
