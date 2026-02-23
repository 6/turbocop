x = 1
y = 2
foo(1, 2)
bar = "hello world"
name      = "RuboCop"
website   = "rubocop.org"
object.method(arg) # this is a comment

# Aligned assignment operators (AllowForAlignment: true)
a   = 1
b   = 2

# Alignment across blank lines
a  = 1

b  = 2

# Alignment across comment-only lines
name    = "one"
# this is a comment
website = "two"

# Aligned trailing comments
x = 1 # first comment
y = 2 # second comment

# Multiline hash (spacing handled by Layout/HashAlignment, not ExtraSpacing)
config = {
  name:      "RuboCop",
  website:   "rubocop.org",
  version:   "1.0"
}

# Compound assignment alignment (e.g. += aligns with =)
retries     += 1
@http_client = http_client
