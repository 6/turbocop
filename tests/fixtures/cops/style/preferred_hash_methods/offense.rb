hash.has_key?(:foo)
     ^^^^^^^^ Style/PreferredHashMethods: Use `Hash#key?` instead of `Hash#has_key?`.

hash.has_value?(42)
     ^^^^^^^^^^ Style/PreferredHashMethods: Use `Hash#value?` instead of `Hash#has_value?`.

{a: 1}.has_key?(:a)
       ^^^^^^^^ Style/PreferredHashMethods: Use `Hash#key?` instead of `Hash#has_key?`.

return unless has_key? key
              ^^^^^^^^ Style/PreferredHashMethods: Use `Hash#key?` instead of `Hash#has_key?`.

unless has_key?(key)
       ^^^^^^^^ Style/PreferredHashMethods: Use `Hash#key?` instead of `Hash#has_key?`.

!has_key? key
 ^^^^^^^^ Style/PreferredHashMethods: Use `Hash#key?` instead of `Hash#has_key?`.

if has_key?(x)
   ^^^^^^^^ Style/PreferredHashMethods: Use `Hash#key?` instead of `Hash#has_key?`.

if has_key?(x)
   ^^^^^^^^ Style/PreferredHashMethods: Use `Hash#key?` instead of `Hash#has_key?`.

has_key?(x)
^^^^^^^^ Style/PreferredHashMethods: Use `Hash#key?` instead of `Hash#has_key?`.

return self[key] if has_key?(key)
                    ^^^^^^^^ Style/PreferredHashMethods: Use `Hash#key?` instead of `Hash#has_key?`.

has_key?(key) ? self[key] : nil
^^^^^^^^ Style/PreferredHashMethods: Use `Hash#key?` instead of `Hash#has_key?`.

has_value?(value)
^^^^^^^^^^ Style/PreferredHashMethods: Use `Hash#value?` instead of `Hash#has_value?`.
