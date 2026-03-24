arr[0]
   ^^^ Style/ArrayFirstLast: Use `first`.

arr[-1]
   ^^^^ Style/ArrayFirstLast: Use `last`.

items[0]
     ^^^ Style/ArrayFirstLast: Use `first`.

# Inside array literal that is argument to []=
hash[key] = [arr[0], records[-1]]
                ^^^ Style/ArrayFirstLast: Use `first`.
                            ^^^^ Style/ArrayFirstLast: Use `last`.

# Compound assignment on indexed access (IndexOperatorWriteNode)
padding[0] += delta
       ^^^ Style/ArrayFirstLast: Use `first`.

line_widths[-1] += width
           ^^^^ Style/ArrayFirstLast: Use `last`.

options[0] += 1
       ^^^ Style/ArrayFirstLast: Use `first`.

# Logical-or assignment on indexed access (IndexOrWriteNode)
params[0] ||= "localhost"
      ^^^ Style/ArrayFirstLast: Use `first`.

colors[-1] ||= "red"
      ^^^^ Style/ArrayFirstLast: Use `last`.

# Logical-and assignment on indexed access (IndexAndWriteNode)
items[0] &&= transform(value)
     ^^^ Style/ArrayFirstLast: Use `first`.

# Explicit method call syntax: arr.[](0)
arr.[](0)
    ^^^^^ Style/ArrayFirstLast: Use `first`.

arr.[](-1)
    ^^^^^^ Style/ArrayFirstLast: Use `last`.

# Safe-navigation explicit method call: arr&.[](0)
arr&.[](0)
     ^^^^^ Style/ArrayFirstLast: Use `first`.

arr&.[](-1)
     ^^^^^^ Style/ArrayFirstLast: Use `last`.
