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

# Single blank line inside =begin/=end block comment is fine.
=begin
some documentation

more documentation
=end
x = 1

# Consecutive blank lines inside =begin/=end are NOT flagged
# (Parser gem doesn't produce tokens for embdoc content).
=begin


lots of space


=end
y = 2

# Single blank line between code and comment is fine.
puts "last code"

# This single blank line above is not an offense.

# Blank lines after the LAST token (including comments) are not checked.
# This comment is the last token line in the file.
puts "done"

# Blank lines before __END__ are not flagged.
e = 5


__END__
data section here
