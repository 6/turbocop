x =~ /\=/
      ^^ Style/RedundantRegexpEscape: Redundant escape of `=` in regexp.

x =~ /\:/
      ^^ Style/RedundantRegexpEscape: Redundant escape of `:` in regexp.

x =~ /\,/
      ^^ Style/RedundantRegexpEscape: Redundant escape of `,` in regexp.

# Inside character class: dot is redundant
x =~ /[\.]/
       ^^ Style/RedundantRegexpEscape: Redundant escape of `.` in regexp.
# Inside character class: plus is redundant
x =~ /[\+]/
       ^^ Style/RedundantRegexpEscape: Redundant escape of `+` in regexp.
# Escaped hyphen at end of character class is redundant
x =~ /[a-z0-9\-]/
             ^^ Style/RedundantRegexpEscape: Redundant escape of `-` in regexp.
