%q('"hi"')

'hello world'

"hello world"

%q(\'foo\')

x = "normal string"

# %Q with both quote kinds is not redundant
%Q(He said "hello" before it's done)

# %Q with escapes that require double quotes is not redundant
%Q(<?xml version="1.0" encoding="UTF-8"?>\n)

# %Q with interpolation AND double quotes is not redundant
%Q("hi#{4}")
%Q(She said "yes" #{name})
