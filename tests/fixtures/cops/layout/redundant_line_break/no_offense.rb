my_method(1, 2, "x")

foo(a, b)

a = if x
      1
    else
      2
    end

foo \
  && bar

foo \
  || bar

x = 42

# Backslash in a comment line should not trigger
# 'foo' \
#   'bar'

# This is a YARD example with backslash \
# continuation that is just a comment

# A line that would be too long when combined (exceeds 120 chars):
this_is_a_very_long_method_name_that_makes_the_line_quite_long(argument_one, argument_two, argument_three) \
  .and_then_another_long_chain_call

MSG = 'This is a long error message string that definitely ' \
      'exceeds one hundred and twenty characters when concatenated together'

# String concatenation with backslash — RuboCop handles these at the AST
# level and checks the full expression context, not just the continuation.
# The text-based approach should skip string literal concatenation.
msg = 'short string that ' \
      'fits on one line'

error = "Node type must be any of #{types}, " \
        "passed #{node_type}"

label = "#{name}::" \
        "#{child_name}"

expect(output)
  .to eq('[modify] A configuration is added into ' \
         "#{path}.\n")

# Method call on a single line is fine
my_method(1, 2, "x")

# Multiline method call that would exceed 120 chars when joined on one line
my_method(1111111111111111,
          2222222222222222,
          3333333333333333,
          4444444444444444,
          5555555555555555,
          6666666666666666,
          7777777777777777)

# Method call with comments on intermediate lines
my_method(1,
          2,
          "x") # X

# Assignment containing an if expression
a =
  if x
    1
  else
    2
  end

# Assignment containing a case expression
a =
  case x
  when :a
    1
  else
    2
  end

# Method call with a do block (InspectBlocks: false by default)
a do
  x
  y
end

# Assignment containing a begin-end expression
a ||= begin
  x
  y
end

# Complex method chain that is too long for a single line
node.each_node(:dstr).select(&:heredoc?).map { |n| n.loc.heredoc_body }.flat_map { |b| (b.line...b.last_line).to_a }

# Method call with heredoc argument
foo(<<~EOS)
  xyz
EOS

# Method call with a multiline string argument
foo('
  xyz
')

# Quoted symbol with a single newline
foo(:"
")

# Binary expression containing an if expression
a +
  if x
    1
  else
    2
  end

# Modified singleton method definition
x def self.y
    z
  end

# Multiline block without a chained method call (InspectBlocks: false)
f do
end

# Method call chained onto a multiline do block (InspectBlocks: false)
e.select do |i|
  i.cond?
end.join

# A method call chained onto a single line block (Layout/SingleLineBlockChain precedence)
e.select { |i| i.cond? }
 .join

# Index access call chained — see RuboCop's index_access_call_chained? check
# hash[:foo] \
#   [:bar]
