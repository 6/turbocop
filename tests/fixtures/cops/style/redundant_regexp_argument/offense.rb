'foo'.gsub(/bar/, 'baz')
           ^^^^^ Style/RedundantRegexpArgument: Use string `"` instead of regexp `/` as the argument.

'foo'.sub(/bar/, 'baz')
          ^^^^^ Style/RedundantRegexpArgument: Use string `"` instead of regexp `/` as the argument.

'foo'.split(/,/)
            ^^^ Style/RedundantRegexpArgument: Use string `"` instead of regexp `/` as the argument.

'foo'.gsub(/\./, '-')
           ^^^^ Style/RedundantRegexpArgument: Use string `"` instead of regexp `/` as the argument.

'foo'.split(/\-/)
            ^^^^ Style/RedundantRegexpArgument: Use string `"` instead of regexp `/` as the argument.

'foo'.sub(/\//, '-')
          ^^^^ Style/RedundantRegexpArgument: Use string `"` instead of regexp `/` as the argument.

# Empty regexp is deterministic
'foo'.split(//)
            ^^ Style/RedundantRegexpArgument: Use string `"` instead of regexp `/` as the argument.

# %r with slash delimiter is deterministic
'foo'.gsub(%r/\./, '-')
           ^^^^^^ Style/RedundantRegexpArgument: Use string `"` instead of regexp `/` as the argument.

# %r with ! delimiter is deterministic
'foo'.split(%r!foo!)
            ^^^^^^ Style/RedundantRegexpArgument: Use string `"` instead of regexp `/` as the argument.
