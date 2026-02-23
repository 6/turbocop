dest = []
src.each { |x| dest << x * 2 }
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/MapIntoArray: Use `map` instead of `each` to map elements into an array.
result = []
items.each { |item| result << item.to_s }
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/MapIntoArray: Use `map` instead of `each` to map elements into an array.
output = []
list.each { |e| output << transform(e) }
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/MapIntoArray: Use `map` instead of `each` to map elements into an array.
