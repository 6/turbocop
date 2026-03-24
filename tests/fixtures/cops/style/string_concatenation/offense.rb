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
