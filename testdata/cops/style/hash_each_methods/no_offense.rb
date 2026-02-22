foo.each_key { |k| p k }
foo.each_value { |v| p v }
keys.each { |k| p k }
values.each { |v| p v }
foo.each { |k, v| do_something(k, v) }
{}.each_key { |k| p k }
# Both args used
foo.each { |k, v| puts "#{k}: #{v}" }
# Both args unused (skip)
foo.each { |_k, _v| puts "hello" }
# Single arg
foo.each { |item| p item }
