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
