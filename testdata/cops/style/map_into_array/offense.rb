src.each { |x| dest << x * 2 }
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/MapIntoArray: Use `map` instead of `each` to map elements into an array.
items.each { |item| result << item.to_s }
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/MapIntoArray: Use `map` instead of `each` to map elements into an array.
list.each { |e| output << transform(e) }
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/MapIntoArray: Use `map` instead of `each` to map elements into an array.
