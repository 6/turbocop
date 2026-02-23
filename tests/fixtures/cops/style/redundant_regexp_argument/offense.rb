'foo'.gsub(/bar/, 'baz')
           ^^^^^ Style/RedundantRegexpArgument: Use string `"` instead of regexp `/` as the argument.

'foo'.sub(/bar/, 'baz')
          ^^^^^ Style/RedundantRegexpArgument: Use string `"` instead of regexp `/` as the argument.

'foo'.split(/,/)
            ^^^ Style/RedundantRegexpArgument: Use string `"` instead of regexp `/` as the argument.
