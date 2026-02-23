[].inject(:+)
[1, 2, 3].inject { |a, e| a + e }
[].inject({}) { |h| h[rand] = rand; h }
array.reduce(0) { |a, e| a }
[].each_with_object({}) { |e, a| a[e] = 1 }
x = 1
[].inject({}) { |h, e| h.merge(e) }
[].inject([]) { |a, e| a + [e] }

# Accumulator reassigned — can't convert to each_with_object
list.reduce([]) do |fields, item|
  fields += [item.to_h]
  fields
end
