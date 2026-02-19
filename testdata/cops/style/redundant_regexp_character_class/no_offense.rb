x =~ /[ab]/
x =~ /[a-z]/
x =~ /[^a]/
x =~ /\d/
x = 'hello'
y = /foo/

# Extended mode — space in char class is needed to match literal space
z = /foo[ ]bar/x
w = /hello[ ]world/x
