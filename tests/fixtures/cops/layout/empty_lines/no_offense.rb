x = 1

y = 2

z = 3
a = 4

b = 5

# Whitespace-only lines are NOT blank according to RuboCop.
# The following lines contain spaces/tabs but are not truly empty.
def foo
  x = 1
  
  
  y = 2
end

# Consecutive blank lines inside a multi-line string are not offenses.
result = "test


                                    string"

# Single blank line between code and comment is fine.
puts "last code"

# This single blank line above is not an offense.

# Blank lines after the LAST token (including comments) are not checked.
# This comment is the last token line in the file.
puts "done"

# Consecutive blank lines inside =begin/=end are not checked after the last token.
x = 1
=begin


lots of space


=end
