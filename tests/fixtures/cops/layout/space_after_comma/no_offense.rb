foo(1, 2)
x = [1, 2, 3]
y = "a,b"
bar(a, b, c)
{a: 1, b: 2}
[1,
 2]
# $, global variable is not a comma separator
def safe_join(array, sep = $,)
end

# Trailing comma in block parameters: |var,| ignores extra values
items.each { |key,| puts key }
items.all? do |detected_sequence,|
  detected_sequence.valid?
end

# Trailing comma before closing delimiters
foo(a, b,)
[1, 2,]
pairs = [['ADJ',	'Adjective']]

# Line continuation: comma before backslash at end of line is OK
add project.permalink, lastmod: project.updated_at,\
                       changefreq: 'monthly'

s_09, s_08, s_07, s_06 = \
(l /= 10; digits[l % 10]), (l /= 10; digits[l % 10]),\
(l /= 10; digits[l % 10]), (l /= 10; digits[l % 10])

# Trailing comma before semicolon in pattern matching
case [0, 1, 2, 3]
in 0, 1,;
  true
end

# Trailing comma before } with space is always OK (any style)
{ foo: bar, }

# Commas inside string literals nested in heredoc interpolation are not code
expected = <<~STR
  #{method('1,2,3')}
STR
output = <<~MSG
  #{colorize("item,value")}
MSG
