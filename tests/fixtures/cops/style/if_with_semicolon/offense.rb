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

# Nested if with semicolon inside parent if with semicolon (RuboCop ignore_node)
# Only the outer if is flagged; inner if is suppressed via part_of_ignored_node?
if is_real?;
^^^^^^^^^^^^ Style/IfWithSemicolon: Do not use `if is_real?;` - use a newline instead.
  if @re>=0; return foo
  else return bar
  end
end

# Nested if with semicolon inside elsif with semicolon
# Only the outer if is flagged; nested ifs are suppressed
if other.kind_of?(Quaternion); ((self.log)*other).exp
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/IfWithSemicolon: Do not use `if other.kind_of?(Quaternion);` - use a newline instead.
elsif other.kind_of?(Integer);
  if other==0; return One
  elsif other>0; x = self
  end
end

# if with semicolon inside case else (not an if's else) — should be flagged
# The `else` here belongs to `case`, not to an `if` node, so
# `node.parent&.if_type?` is false in RuboCop.
case tt
when :slash then slt = tt
else if at; zt = tt; else; at = tt; end
     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/IfWithSemicolon: Do not use `if at;` - use a newline instead.
end

# Standalone `;` on next line with empty if-body — Prism Translation Parser
# treats the `;` as loc.begin (then-keyword), so RuboCop flags this.
if params[:layer] == '*' and query[:resource] == :objects
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/IfWithSemicolon: Do not use `if params[:layer] == '*' and query[:resource] == :objects;` - use a newline instead.
  ;
else
  process
end
