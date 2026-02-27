x =~ /[ab]/
x =~ /[a-z]/
x =~ /[^a]/
x =~ /\d/
x = 'hello'
y = /foo/

# Extended mode — space in char class is needed to match literal space
z = /foo[ ]bar/x
w = /hello[ ]world/x

# Regex metacharacters — character class is a valid escape technique
x =~ /[.]\d+/
x =~ /[*]\s/
x =~ /[?]/
x =~ /[+]/
x =~ /[(][?][:=<!]/
x =~ /[)]/
x =~ /[{][\d,]+[}]/
x =~ /[|]/
x =~ /[^]/
x =~ /[$]/
x = %r([*].*?[*]/)
x = %r/[{]\d+[}]/
