r = ('A'..'z')
     ^^^^^^^^ Lint/MixedCaseRange: Ranges from upper to lower case ASCII letters may include unintended characters. Instead of `A-z` (which also includes several symbols) specify each range individually: `A-Za-z` and individually specify any symbols.
x = ('a'..'Z')
     ^^^^^^^^ Lint/MixedCaseRange: Ranges from upper to lower case ASCII letters may include unintended characters. Instead of `A-z` (which also includes several symbols) specify each range individually: `A-Za-z` and individually specify any symbols.
y = ('B'..'f')
     ^^^^^^^^ Lint/MixedCaseRange: Ranges from upper to lower case ASCII letters may include unintended characters. Instead of `A-z` (which also includes several symbols) specify each range individually: `A-Za-z` and individually specify any symbols.
