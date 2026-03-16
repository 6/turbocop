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

# In always-indented context (lambda body), second line should be indented
x = ->(obj) {
  "line one" \
  "line two"
  ^^^^^^^^^^ Layout/LineEndStringConcatenationIndentation: Indent the first part of a string concatenated with backslash.
}

# Operator assignment inside def - NOT always-indented, parent is op_asgn
def update_record
  msg = "initial"
  msg +=
    "first part " \
      "second part"
      ^^^^^^^^^^^^^ Layout/LineEndStringConcatenationIndentation: Align parts of a string concatenated with backslash.
end

# Index operator assignment inside def - NOT always-indented
def process_errors
  errors[:detail] +=
    "a valid item has a single " \
      "root directory"
      ^^^^^^^^^^^^^^^^ Layout/LineEndStringConcatenationIndentation: Align parts of a string concatenated with backslash.
end

# Call operator write (x.y +=) inside def
def handle_response
  response.body +=
    "extra content " \
      "appended here"
      ^^^^^^^^^^^^^^^ Layout/LineEndStringConcatenationIndentation: Align parts of a string concatenated with backslash.
end

# Or-assignment inside def
def set_default
  @value ||=
    "default " \
      "value"
      ^^^^^^^ Layout/LineEndStringConcatenationIndentation: Align parts of a string concatenated with backslash.
end

# In always-indented context (if branch), aligned dstr should be indented
# (previously a false negative)
def show_message
  str = ""
  str << if condition
           "The first part of a long " \
           "message that spans multiple lines."
           ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Layout/LineEndStringConcatenationIndentation: Indent the first part of a string concatenated with backslash.
         else
           "A different " \
           "message here."
           ^^^^^^^^^^^^^^^ Layout/LineEndStringConcatenationIndentation: Indent the first part of a string concatenated with backslash.
         end
end

# In parenthesized expression (Parser :begin, always-indented), aligned dstr should be indented
def build_message
  result << (
    'fail validation if '\
    ":#{name} is unset; "\
    ^^^^^^^^^^^^^^^^^^^^^^^^^^^ Layout/LineEndStringConcatenationIndentation: Indent the first part of a string concatenated with backslash.
    'should have been defined'
  )
end

# In explicit begin...end (Parser :kwbegin, NOT always-indented), aligned check
def cache_key_prefix
  @prefix ||= begin
    indicator = unique ? "" : "s"
    "attr#{indicator}" \
      ":#{model_name}" \
      ^^^^^^^^^^^^^^^^^^^^^ Layout/LineEndStringConcatenationIndentation: Align parts of a string concatenated with backslash.
      ":#{attribute}"
  end
end
