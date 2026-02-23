foo.reduce { |acc, elem| acc + elem }
foo.inject { |acc, elem| acc + elem }
foo.map { |x| x * 2 }
foo.each { |item| puts item }
foo.reduce do |a, b|
  a + b
end
x = [1, 2, 3]
