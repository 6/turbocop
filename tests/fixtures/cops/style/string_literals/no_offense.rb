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

# Unicode escape sequences need double quotes
copyright = "\u00A9"
hex_str = "\xf9"

# Control character escapes need double quotes
esc = "\e"

# %q, %Q, and % strings should be ignored
a = %q(hello)
b = %Q[world]
c = %(test)

# Character literal should be ignored
d = ?/

# String with escaped hash — \# is different from # in double quotes
e = "\#"

# Double-quoted strings inside interpolation should not be flagged
# (RuboCop skips strings inside #{ } interpolation for both styles)
msg = "hello #{data["key"]}"
log = "value: #{record.dig("a", "b")}"
out = "#{items.join(", ")}"
