str[0]
str[-1]
str.chars
str.chars.count
str.chars.map { |c| c.upcase }
# .chars.last(n) is not equivalent to a simple string slice for edge cases
str.chars.last(40).join
result.chars.last(5).join
