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

exif[0]&.raw_fields&.[](BORDER_TAG_IDS[border])&.[](0)
    ^^^ Style/ArrayFirstLast: Use `first`.

assert_equal "hello", result[0].content[0][:text]
                            ^^^ Style/ArrayFirstLast: Use `first`.

assert_equal "world", result[0].content[1][:text]
                            ^^^ Style/ArrayFirstLast: Use `first`.

inner_doc = doc.blocks[0].rows.body[0][0].inner_document
                      ^^^ Style/ArrayFirstLast: Use `first`.

cell = (document_from_string input).blocks[0].rows.body[0][0]
                                          ^^^ Style/ArrayFirstLast: Use `first`.

dd = doc.blocks[0].items[0][1]
               ^^^ Style/ArrayFirstLast: Use `first`.

result[pair.children[0].children[0]] = Solargraph::Parser.chain(pair.children[1])
                    ^^^ Style/ArrayFirstLast: Use `first`.

credential[:tokentype] = tokentype[0].split(":")[1]
                                  ^^^ Style/ArrayFirstLast: Use `first`.
