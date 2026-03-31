r = /[xyx]/
        ^ Lint/DuplicateRegexpCharacterClassElement: Duplicate element inside regexp character class
r = /[aba]/
        ^ Lint/DuplicateRegexpCharacterClassElement: Duplicate element inside regexp character class
r = /[1231]/
         ^ Lint/DuplicateRegexpCharacterClassElement: Duplicate element inside regexp character class
# Duplicate single quotes in character class
r = /["'']/
        ^ Lint/DuplicateRegexpCharacterClassElement: Duplicate element inside regexp character class
# Duplicate in interpolated regex
r = /["'']?.*foo/
        ^ Lint/DuplicateRegexpCharacterClassElement: Duplicate element inside regexp character class
r = /[A-Aa-z0-9]+/
        ^ Lint/DuplicateRegexpCharacterClassElement: Duplicate element inside regexp character class

/^([[#{Regexp.escape(exclude_item)}(?:,.*?)?]])\s*$/,           # [[id]] or [[id,ref...]]
                                         ^ Lint/DuplicateRegexpCharacterClassElement: Duplicate element inside regexp character class
                                           ^ Lint/DuplicateRegexpCharacterClassElement: Duplicate element inside regexp character class

r = /[[a][a]]/
         ^ Lint/DuplicateRegexpCharacterClassElement: Duplicate element inside regexp character class
