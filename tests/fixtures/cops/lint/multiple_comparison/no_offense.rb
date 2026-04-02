x < y && y < z
10 <= x && x <= 20
a > b
x < y
a >= b && b <= c
x == y
a != b
x < y || y > z
min <= value && value <= max

# Set operations as center value should not be flagged
x >= y & x < z
x >= y | x < z
x >= y ^ x < z

# Overloaded operator methods chained with blocks/lambdas are not comparisons
either = Success(1).
  >= {|prev| Success(prev + 1) }.
  >= -> prev { Success(prev + 100) }

Either(Maybe(params)['id']).or('id is missing').
  >= {|v| Try { BSONTestConverter.from_string(v) }.or("id '#{v}' is not a valid BSON id") }.
  >= {|v| Try { DatabaseReader.find(v) }.or("'#{v}' not found") }
