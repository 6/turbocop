str.split(/,/)
          ^^^ Performance/RedundantSplitRegexpArgument: Use string as argument instead of regexp.
str.split(/-/)
          ^^^ Performance/RedundantSplitRegexpArgument: Use string as argument instead of regexp.
str.split(/\./)
          ^^^^ Performance/RedundantSplitRegexpArgument: Use string as argument instead of regexp.
str.split(/\n/)
          ^^^^ Performance/RedundantSplitRegexpArgument: Use string as argument instead of regexp.
str.split(/\t/)
          ^^^^ Performance/RedundantSplitRegexpArgument: Use string as argument instead of regexp.
str.split(//)
          ^^ Performance/RedundantSplitRegexpArgument: Use string as argument instead of regexp.
str.split(/  /)
          ^^^^ Performance/RedundantSplitRegexpArgument: Use string as argument instead of regexp.
str&.split(/,/)
           ^^^ Performance/RedundantSplitRegexpArgument: Use string as argument instead of regexp.
body.split(/\&/)
           ^^^^ Performance/RedundantSplitRegexpArgument: Use string as argument instead of regexp.
str.split(/\n\n/)
          ^^^^^^ Performance/RedundantSplitRegexpArgument: Use string as argument instead of regexp.
# Regex with /o flag (once-only) — does not affect literal matching
str.split(/:/o)
          ^^^^ Performance/RedundantSplitRegexpArgument: Use string as argument instead of regexp.
# Regex with /m flag (multiline) — does not affect literal matching
str.split(/\n\n/m)
          ^^^^^^^ Performance/RedundantSplitRegexpArgument: Use string as argument instead of regexp.
str.split(/\r\n\|\r\n/m)
          ^^^^^^^^^^^^^^ Performance/RedundantSplitRegexpArgument: Use string as argument instead of regexp.
# Regex with /x flag — does not affect simple literal matching
str.split(/,/x)
          ^^^^ Performance/RedundantSplitRegexpArgument: Use string as argument instead of regexp.
