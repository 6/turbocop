x = "hello"
    ^ Style/StringLiterals: Prefer single-quoted strings when you don't need string interpolation or special symbols.
y = "world"
    ^ Style/StringLiterals: Prefer single-quoted strings when you don't need string interpolation or special symbols.
z = "foo bar"
    ^ Style/StringLiterals: Prefer single-quoted strings when you don't need string interpolation or special symbols.
u = "has \\ backslash"
    ^ Style/StringLiterals: Prefer single-quoted strings when you don't need string interpolation or special symbols.
a = "\\"
    ^ Style/StringLiterals: Prefer single-quoted strings when you don't need string interpolation or special symbols.
b = "\""
    ^ Style/StringLiterals: Prefer single-quoted strings when you don't need string interpolation or special symbols.
c = "España"
    ^ Style/StringLiterals: Prefer single-quoted strings when you don't need string interpolation or special symbols.
# Strings with only \" escapes can use single quotes (\" becomes literal " in single quotes)
d = "execve(\"/bin/sh\", rsp, environ)"
    ^ Style/StringLiterals: Prefer single-quoted strings when you don't need string interpolation or special symbols.
e = "{\"key\": \"value\"}"
    ^ Style/StringLiterals: Prefer single-quoted strings when you don't need string interpolation or special symbols.
