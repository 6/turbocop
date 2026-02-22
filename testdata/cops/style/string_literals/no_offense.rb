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

# Strings with undefined escape sequences like \g — RuboCop treats any
# backslash-escape (except \\ and \") as requiring double quotes
desc = "with a regexp containing invalid \g escape"
note = "with an invalid \p pattern"
