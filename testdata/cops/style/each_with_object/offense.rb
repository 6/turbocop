[].inject({}) { |a, e| a }
   ^^^^^^ Style/EachWithObject: Use `each_with_object` instead of `inject`.

[].reduce({}) { |a, e| a }
   ^^^^^^ Style/EachWithObject: Use `each_with_object` instead of `reduce`.

[1, 2, 3].inject({}) do |h, i|
          ^^^^^^ Style/EachWithObject: Use `each_with_object` instead of `inject`.
  h[i] = i
  h
end
