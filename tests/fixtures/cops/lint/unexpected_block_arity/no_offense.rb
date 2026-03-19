values.reduce { |memo, obj| memo << obj }
values.inject { |memo, obj| memo + obj }
values.reduce { |*args| args }
values.map { |x| x }
values.each { |x| x }
values.reduce { _1 + _2 }
values.reduce { _1 + _2 + _3 }
values.reduce { _2 }
values.reduce { |(a, b), c| a + b + c }
values.reduce { |a = 1, b = 2| a + b }
values.reduce { |a, b, c| a + b }
reduce { }
reduce { _1 }
reduce { it }
values.each_with_index { |elem, idx| elem }
values.each_with_object([]) { |v, memo| memo << v }
values.sort { |a, b| a <=> b }
values.chunk_while { |a, b| a == b }
values.slice_when { |a, b| a != b }
values.max { |a, b| a <=> b }
values.min { |a, b| a <=> b }
values.minmax { |a, b| a <=> b }
