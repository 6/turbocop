text = 'hello' \
    'world'
    ^^^^^^^ Layout/LineEndStringConcatenationIndentation: Align parts of a string concatenated with backslash.

result = "one" \
           "two"
           ^^^^^ Layout/LineEndStringConcatenationIndentation: Align parts of a string concatenated with backslash.

# In always-indented context (def body), second line should be indented
def some_method
  'x' \
  'y' \
  ^^^ Layout/LineEndStringConcatenationIndentation: Indent the first part of a string concatenated with backslash.
  'z'
end

# In always-indented context (block body), second line should be indented
foo do
  "hello" \
  "world"
  ^^^^^^^ Layout/LineEndStringConcatenationIndentation: Indent the first part of a string concatenated with backslash.
end
