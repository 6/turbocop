(1..4).reduce(0) do |acc, el|
  acc + el
end
(1..4).reduce(0) do |acc, el|
  acc
end
(1..4).reduce(0) do |acc, el|
  acc += el
end
(1..4).reduce(0) do |acc, el|
  acc << el
end
values.reduce(:+)
values.reduce do
  do_something
end
foo.reduce { |result, key| result.method(key) }
