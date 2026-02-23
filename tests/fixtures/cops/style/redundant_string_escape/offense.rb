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
