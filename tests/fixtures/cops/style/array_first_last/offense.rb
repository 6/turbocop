arr[0]
   ^^^ Style/ArrayFirstLast: Use `first`.

arr[-1]
   ^^^^ Style/ArrayFirstLast: Use `last`.

items[0]
     ^^^ Style/ArrayFirstLast: Use `first`.

# Inside array literal that is argument to []=
hash[key] = [arr[0], records[-1]]
                ^^^ Style/ArrayFirstLast: Use `first`.
                            ^^^^ Style/ArrayFirstLast: Use `last`.
