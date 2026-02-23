foo.keys.each { |k| p k }
    ^^^^^^^^^ Style/HashEachMethods: Use `each_key` instead of `keys.each`.
foo.values.each { |v| p v }
    ^^^^^^^^^^^ Style/HashEachMethods: Use `each_value` instead of `values.each`.
{}.keys.each { |k| p k }
   ^^^^^^^^^ Style/HashEachMethods: Use `each_key` instead of `keys.each`.
{}.values.each { |k| p k }
   ^^^^^^^^^^^ Style/HashEachMethods: Use `each_value` instead of `values.each`.
opts.each { |key, _directory| p key }
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/HashEachMethods: Use `each_key` instead of `each` and remove the unused `_directory` block argument.
settings.each { |key, _| p key }
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/HashEachMethods: Use `each_key` instead of `each` and remove the unused `_` block argument.
data.each { |_k, val| p val }
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/HashEachMethods: Use `each_value` instead of `each` and remove the unused `_k` block argument.
