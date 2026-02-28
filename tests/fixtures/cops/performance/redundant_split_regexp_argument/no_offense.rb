str.split(',')
str.split(' ')
str.split
str.split(variable)
split(/,/)
# Regex with character class — not a simple string replacement
str.split(/[_-]/)
str.split(/\s+/)
str.split(/[,;]/)
# Single space regexp is NOT equivalent to " " (preserves whitespace literally)
str.split(/ /)
# split with a limit argument — regexp is not redundant
str.split(/ /, 3)
str.split(/,/, 2)
str.split(/-/, -1)
# Regex features that are not simple literals
str.split(/\d+/)
str.split(/\w/)
str.split(/\b/)
# %r{} syntax — RuboCop does not flag these
str.split(%r{,})
str.split(%r{=})
pair.split(%r{=})
body.split(%r{&})
# Regexp with flags — not a simple replacement
str.split(/split/i)
str.split(/,/x)
