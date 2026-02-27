x =~ /[x]/
      ^^^ Style/RedundantRegexpCharacterClass: Redundant single-element character class, `[x]` can be replaced with `x`.

x =~ /[\d]/
      ^^^^ Style/RedundantRegexpCharacterClass: Redundant single-element character class, `[x]` can be replaced with `x`.

x =~ /[a]b[c]d/
      ^^^ Style/RedundantRegexpCharacterClass: Redundant single-element character class, `[x]` can be replaced with `x`.
          ^^^ Style/RedundantRegexpCharacterClass: Redundant single-element character class, `[x]` can be replaced with `x`.

x =~ /([a])/
       ^^^ Style/RedundantRegexpCharacterClass: Redundant single-element character class, `[x]` can be replaced with `x`.
