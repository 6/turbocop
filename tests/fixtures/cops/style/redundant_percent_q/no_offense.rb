%q('"hi"')

'hello world'

"hello world"

%q(\'foo\')

x = "normal string"

# %Q with double quotes inside is not redundant (would need escaping in "")
%Q(<?xml version="1.0" encoding="UTF-8"?>\n)
%Q(He said "hello" to me)
