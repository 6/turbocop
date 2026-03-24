# Single-line if with semicolon
if foo; bar end
^^^^^^^^^^^^^^^ Style/IfWithSemicolon: Do not use `if foo;` - use a newline instead.

if foo; bar else baz end
^^^^^^^^^^^^^^^^^^^^^^^^ Style/IfWithSemicolon: Do not use `if foo;` - use a newline instead.

if condition; do_something end
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/IfWithSemicolon: Do not use `if condition;` - use a newline instead.

# Multi-line if with semicolon after condition (body on next line)
if true;
^^^^^^^^ Style/IfWithSemicolon: Do not use `if true;` - use a newline instead.
  do_something
end

# Unless with semicolon, multi-line
unless done;
^^^^^^^^^^^^ Style/IfWithSemicolon: Do not use `unless done;` - use a newline instead.
  process
end

# Multi-line if with semicolon and parenthesized condition
if (97 <= cc && cc <= 122);
^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/IfWithSemicolon: Do not use `if (97 <= cc && cc <= 122);` - use a newline instead.
  return true
end

# Trailing semicolon with simple parenthesized condition
if (octets);
^^^^^^^^^^^^ Style/IfWithSemicolon: Do not use `if (octets);` - use a newline instead.
  index = process(octets, result, index)
end
