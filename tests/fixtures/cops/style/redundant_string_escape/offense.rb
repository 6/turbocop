"foo\=bar"
    ^^ Style/RedundantStringEscape: Redundant escape of `=` in string.

"foo\:bar"
    ^^ Style/RedundantStringEscape: Redundant escape of `:` in string.

"hello\,world"
      ^^ Style/RedundantStringEscape: Redundant escape of `,` in string.

"it\'s here"
   ^^ Style/RedundantStringEscape: Redundant escape of `'` in string.

"foo\'bar\'baz"
    ^^ Style/RedundantStringEscape: Redundant escape of `'` in string.
         ^^ Style/RedundantStringEscape: Redundant escape of `'` in string.

"\#foo"
 ^^ Style/RedundantStringEscape: Redundant escape of `#` in string.

"test\#value"
     ^^ Style/RedundantStringEscape: Redundant escape of `#` in string.

"foo #{bar} \' baz"
            ^^ Style/RedundantStringEscape: Redundant escape of `'` in string.

"foo\{bar"
    ^^ Style/RedundantStringEscape: Redundant escape of `{` in string.

"\#\{foo}"
   ^^ Style/RedundantStringEscape: Redundant escape of `{` in string.

<<~STR
  foo\"bar
     ^^ Style/RedundantStringEscape: Redundant escape of `"` in string.
STR

<<~STR
  foo\'bar
     ^^ Style/RedundantStringEscape: Redundant escape of `'` in string.
STR

<<-HEREDOC
  test\#value
      ^^ Style/RedundantStringEscape: Redundant escape of `#` in string.
HEREDOC

%(foo\"bar)
     ^^ Style/RedundantStringEscape: Redundant escape of `"` in string.

%(foo\.bar)
     ^^ Style/RedundantStringEscape: Redundant escape of `.` in string.

%Q(foo\"bar)
      ^^ Style/RedundantStringEscape: Redundant escape of `"` in string.

%Q!foo\'bar!
      ^^ Style/RedundantStringEscape: Redundant escape of `'` in string.

%W[\" ']
   ^^ Style/RedundantStringEscape: Redundant escape of `"` in string.

"\“#{locale}\”"
 ^^ Style/RedundantStringEscape: Redundant escape of `“` in string.
            ^^ Style/RedundantStringEscape: Redundant escape of `”` in string.

<<~STR
  line1
  hello\ world
       ^^ Style/RedundantStringEscape: Redundant escape of ` ` in string.
STR

<<-HEREDOC
  line1
  hello\ world
       ^^ Style/RedundantStringEscape: Redundant escape of ` ` in string.
  line3
HEREDOC
