x = 1


^ Layout/EmptyLines: Extra blank line detected.
y = 2


^ Layout/EmptyLines: Extra blank line detected.

^ Layout/EmptyLines: Extra blank line detected.
z = 3

# Consecutive blank lines between code and trailing comments
a = 1


^ Layout/EmptyLines: Extra blank line detected.
# trailing comment

b = 2


^ Layout/EmptyLines: Extra blank line detected.
# another comment

# Consecutive blank lines in a comment-only file
# frozen_string_literal: true


^ Layout/EmptyLines: Extra blank line detected.
# Another comment

# Consecutive blank lines before =begin (FN fix)
c = 3


^ Layout/EmptyLines: Extra blank line detected.
=begin
some docs
=end

# Consecutive blank lines before =begin, no code after =end
d = 4


^ Layout/EmptyLines: Extra blank line detected.

^ Layout/EmptyLines: Extra blank line detected.
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
