foo.keys.each { |k| p k }
    ^^^^^^^^^ Style/HashEachMethods: Use `each_key` instead of `keys.each`.
foo.values.each { |v| p v }
    ^^^^^^^^^^^ Style/HashEachMethods: Use `each_value` instead of `values.each`.
{}.keys.each { |k| p k }
   ^^^^^^^^^ Style/HashEachMethods: Use `each_key` instead of `keys.each`.
{}.values.each { |k| p k }
   ^^^^^^^^^^^ Style/HashEachMethods: Use `each_value` instead of `values.each`.
