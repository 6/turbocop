'foo'.gsub(/bar/, 'baz')
            ^^^^^ Style/RedundantRegexpArgument: Use string `"` instead of regexp `/` as the argument.

'foo'.match(/bar/)
            ^^^^^ Style/RedundantRegexpArgument: Use string `"` instead of regexp `/` as the argument.

'foo'.split(/,/)
            ^^^ Style/RedundantRegexpArgument: Use string `"` instead of regexp `/` as the argument.
