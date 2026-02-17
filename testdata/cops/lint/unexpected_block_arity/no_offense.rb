values.reduce { |memo, obj| memo << obj }
values.inject { |memo, obj| memo + obj }
values.reduce { |*args| args }
values.map { |x| x }
values.each { |x| x }
