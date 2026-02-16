items.each do |x| puts x end
           ^^ Style/BlockDelimiters: Prefer `{...}` over `do...end` for single-line blocks.

items.map {
          ^ Style/BlockDelimiters: Prefer `do...end` over `{...}` for multi-line blocks.
  |x| x * 2
}

[1, 2].each do |i| i + 1 end
            ^^ Style/BlockDelimiters: Prefer `{...}` over `do...end` for single-line blocks.
