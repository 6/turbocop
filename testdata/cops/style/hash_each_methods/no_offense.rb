foo.each_key { |k| p k }
foo.each_value { |v| p v }
keys.each { |k| p k }
values.each { |v| p v }
foo.each { |k, v| do_something(k, v) }
{}.each_key { |k| p k }
