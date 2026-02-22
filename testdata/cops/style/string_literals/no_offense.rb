x = 'hello'
y = "it's got a quote"
z = "has a \n newline"
w = 'simple'
v = "has \t tab"
t = 'another single'
# Multi-line double-quoted string without interpolation or escapes
# should not be flagged (RuboCop skips multi-line strings)
sql = "SELECT * FROM foo
       WHERE bar = baz"
