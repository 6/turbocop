hash.has_key?(:foo)
     ^^^^^^^^ Style/PreferredHashMethods: Use `Hash#key?` instead of `Hash#has_key?`.

hash.has_value?(42)
     ^^^^^^^^^^ Style/PreferredHashMethods: Use `Hash#value?` instead of `Hash#has_value?`.

{a: 1}.has_key?(:a)
       ^^^^^^^^ Style/PreferredHashMethods: Use `Hash#key?` instead of `Hash#has_key?`.
