foo do |x| x end
    ^^ Style/SingleLineDoEndBlock: Prefer braces `{...}` over `do...end` for single-line blocks.

bar do puts 'hello' end
    ^^ Style/SingleLineDoEndBlock: Prefer braces `{...}` over `do...end` for single-line blocks.

baz do |a, b| a + b end
    ^^ Style/SingleLineDoEndBlock: Prefer braces `{...}` over `do...end` for single-line blocks.
