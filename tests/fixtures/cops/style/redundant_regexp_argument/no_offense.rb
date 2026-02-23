'foo'.gsub('bar', 'baz')
'foo'.match(/ba./)
'foo'.split(/,+/)
'foo'.gsub(/\d/, '')
x = 'hello'
y = x.match?('world')

# match and match? are not flagged (not in target methods)
'foo'.match(/bar/)
'foo'.match?(/bar/)

# end_with? not flagged
'foo'.end_with?(/bar/)

# Single space regexp is idiomatic
'foo'.split(/ /)
