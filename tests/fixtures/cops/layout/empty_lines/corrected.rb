x = 1

y = 2

z = 3

# Consecutive blank lines between code and trailing comments
a = 1

# trailing comment

b = 2

# another comment

# Consecutive blank lines in a comment-only file
# frozen_string_literal: true

# Another comment

# Consecutive blank lines before =begin (FN fix)
c = 3

=begin
some docs
=end

# Consecutive blank lines before =begin, no code after =end
d = 4

=begin
more docs
=end

# Blank lines inside =begin/=end are NOT flagged (Parser gem has no embdoc tokens)
e = 5
=begin
some documentation


more documentation
=end
f = 6

g = 7
=begin
docs here



more docs here
=end
