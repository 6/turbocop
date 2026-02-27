foo { |a, b, | a + b }
           ^ Style/TrailingCommaInBlockArgs: Useless trailing comma present in block arguments.

baz { |item, val, | item.to_s }
                ^ Style/TrailingCommaInBlockArgs: Useless trailing comma present in block arguments.

test do |a, b,|
             ^ Style/TrailingCommaInBlockArgs: Useless trailing comma present in block arguments.
  a + b
end

lambda { |foo, bar,| do_something(foo, bar) }
                  ^ Style/TrailingCommaInBlockArgs: Useless trailing comma present in block arguments.
