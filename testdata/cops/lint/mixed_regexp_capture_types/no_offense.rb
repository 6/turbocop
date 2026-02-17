/(?<foo>bar)(?<baz>qux)/
/(foo)(bar)/
/(?<foo>bar)(?:baz)/
/(?<=>)(<br>)(?=><)/
/no captures here/
# Named capture with parens inside a character class (not a capture group)
/\$(?<cmd>\((?:[^()]|\g<cmd>)+\))/
