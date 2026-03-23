r = ('A'..'z')
     ^^^^^^^^ Lint/MixedCaseRange: Ranges from upper to lower case ASCII letters may include unintended characters. Instead of `A-z` (which also includes several symbols) specify each range individually: `A-Za-z` and individually specify any symbols.
x = ('a'..'Z')
     ^^^^^^^^ Lint/MixedCaseRange: Ranges from upper to lower case ASCII letters may include unintended characters. Instead of `A-z` (which also includes several symbols) specify each range individually: `A-Za-z` and individually specify any symbols.
y = ('B'..'f')
     ^^^^^^^^ Lint/MixedCaseRange: Ranges from upper to lower case ASCII letters may include unintended characters. Instead of `A-z` (which also includes several symbols) specify each range individually: `A-Za-z` and individually specify any symbols.

re = /[A-z]/
       ^^^ Lint/MixedCaseRange: Ranges from upper to lower case ASCII letters may include unintended characters. Instead of `A-z` (which also includes several symbols) specify each range individually: `A-Za-z` and individually specify any symbols.

chars = /[a-zA-z0-9]{0,32}/
             ^^^ Lint/MixedCaseRange: Ranges from upper to lower case ASCII letters may include unintended characters. Instead of `A-z` (which also includes several symbols) specify each range individually: `A-Za-z` and individually specify any symbols.

regexp = /[#{prefix}A-z#{suffix}]/
                    ^^^ Lint/MixedCaseRange: Ranges from upper to lower case ASCII letters may include unintended characters. Instead of `A-z` (which also includes several symbols) specify each range individually: `A-Za-z` and individually specify any symbols.

POTENTIAL_BYTES = (' '..'z').to_a
                   ^^^^^^^^ Lint/MixedCaseRange: Ranges from upper to lower case ASCII letters may include unintended characters. Instead of `A-z` (which also includes several symbols) specify each range individually: `A-Za-z` and individually specify any symbols.

PRINTABLE = ("!".."9").to_a + (':'..'Z').to_a + ('['..'z').to_a + ('{'..'~').to_a
                               ^^^^^^^^ Lint/MixedCaseRange: Ranges from upper to lower case ASCII letters may include unintended characters. Instead of `A-z` (which also includes several symbols) specify each range individually: `A-Za-z` and individually specify any symbols.
                                                 ^^^^^^^^ Lint/MixedCaseRange: Ranges from upper to lower case ASCII letters may include unintended characters. Instead of `A-z` (which also includes several symbols) specify each range individually: `A-Za-z` and individually specify any symbols.

chars  = ("\x21".."\x5A").to_a
          ^^^^^^^^^^^^^^ Lint/MixedCaseRange: Ranges from upper to lower case ASCII letters may include unintended characters. Instead of `A-z` (which also includes several symbols) specify each range individually: `A-Za-z` and individually specify any symbols.

CHARS = ('0'..'z').to_a
         ^^^^^^^^ Lint/MixedCaseRange: Ranges from upper to lower case ASCII letters may include unintended characters. Instead of `A-z` (which also includes several symbols) specify each range individually: `A-Za-z` and individually specify any symbols.
