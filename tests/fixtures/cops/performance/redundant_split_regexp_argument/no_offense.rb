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
