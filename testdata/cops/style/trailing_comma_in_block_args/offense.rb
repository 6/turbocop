foo { |a, b, | a + b }
           ^ Style/TrailingCommaInBlockArgs: Useless trailing comma present in block arguments.

bar { |x, | x }
        ^ Style/TrailingCommaInBlockArgs: Useless trailing comma present in block arguments.

baz { |item, | item.to_s }
           ^ Style/TrailingCommaInBlockArgs: Useless trailing comma present in block arguments.
