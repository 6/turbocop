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
