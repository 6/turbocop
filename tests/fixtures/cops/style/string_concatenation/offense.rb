'Hello' + name
^^^^^^^^^^^^^^ Style/StringConcatenation: Prefer string interpolation to string concatenation.

"foo" + "bar"
^^^^^^^^^^^^^ Style/StringConcatenation: Prefer string interpolation to string concatenation.

'prefix_' + value.to_s
^^^^^^^^^^^^^^^^^^^^^^ Style/StringConcatenation: Prefer string interpolation to string concatenation.

# Chain: one offense for the whole chain (at innermost string-concat node)
user.name + ' <' + user.email + '>'
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/StringConcatenation: Prefer string interpolation to string concatenation.

# Chain where only the RHS is a string — fires once at topmost
a + b + 'c'
^^^^^^^^^^^ Style/StringConcatenation: Prefer string interpolation to string concatenation.

# Chain where only the LHS is a string — fires once at innermost
'a' + b + c
^^^^^^^^^^^ Style/StringConcatenation: Prefer string interpolation to string concatenation.

# Mixed chain: string deep in receiver, string at end
a + 'b' + c + 'd'
^^^^^^^^^^^^^^^^^^ Style/StringConcatenation: Prefer string interpolation to string concatenation.

# Single non-literal + string (aggressive mode)
Pathname.new('/') + 'test'
^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/StringConcatenation: Prefer string interpolation to string concatenation.

# Heredoc with single-line content (str in Parser) — flagged
code = <<EOM + extra_code
       ^^^^^^^^^^^^^^^^^^^^ Style/StringConcatenation: Prefer string interpolation to string concatenation.
content
EOM

# Single-line string with escape \n (not multi-line source) — flagged
"hello\nworld" + name
^^^^^^^^^^^^^^^^^^^^^ Style/StringConcatenation: Prefer string interpolation to string concatenation.

# Percent literal %q (single-quoted, str_type? in Parser) — flagged
%q[hello] + %q[world]
^^^^^^^^^^^^^^^^^^^^^ Style/StringConcatenation: Prefer string interpolation to string concatenation.

# Percent literal %() with no interpolation — str_type? in Parser — flagged
name + %(suffix)
^^^^^^^^^^^^^^^^ Style/StringConcatenation: Prefer string interpolation to string concatenation.

# Interpolated string + percent literal — flagged (RHS is str_type?)
"hello #{name}" + %(world)
^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/StringConcatenation: Prefer string interpolation to string concatenation.

# Percent literal %{} with no interpolation — str_type? in Parser — flagged
config + %{some value}
^^^^^^^^^^^^^^^^^^^^^^ Style/StringConcatenation: Prefer string interpolation to string concatenation.

# Percent literal %[] with no interpolation — str_type? in Parser — flagged
header + %[some value]
^^^^^^^^^^^^^^^^^^^^^^ Style/StringConcatenation: Prefer string interpolation to string concatenation.

raise 'Cannot specify both a hash/array/struct and a ' + \
      ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/StringConcatenation: Prefer string interpolation to string concatenation.
  'proc for method #insert!'

__FILE__ + ":#{__LINE__}:in `bar`"
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/StringConcatenation: Prefer string interpolation to string concatenation.
