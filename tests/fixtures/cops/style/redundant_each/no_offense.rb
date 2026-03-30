array.each { |v| do_something(v) }
array.each_with_index { |v, i| do_something(v, i) }
array.each_with_object([]) { |v, o| do_something(v, o) }
array.map.each { |v| v }
array.select.each { |v| v }
array.each
node.each_ancestor(:def, :defs, :block).each do |ancestor|
end
array.each { |i| foo(i) }.each_with_object([]) { |i| bar(i) }
array.each(&:foo).each do |i|
end
array.reverse_each(&:foo).each { |i| bar(i) }
