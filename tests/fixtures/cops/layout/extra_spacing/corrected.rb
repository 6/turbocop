set_app("RuboCop")
website = "https://github.com/rubocop"

x = 1

method_call(arg1, arg2)

# Alignment where adjacent token is NOT preceded by space (coincidental vertical alignment)
d_is_vertically_aligned do
  _______________________d
end

# Extra space before a float in multiline array
{:a => "a",
 :b => [nil, 2.5]}

# Extra spacing in class inheritance
class A < String
end

# Extra spacing before a unary plus in an argument list
assert_difference(MyModel.count, +2,
                  3, +3,
                  4,+4)

# Single-line hash with extra spaces
hash = {a: 1, b: 2}

# Trailing comments at different columns - NOT aligned, should be flagged
check_a_pattern_result # comment A
check_b # comment B
check_c_patterns # comment C

# Extra spaces inside empty word arrays (RuboCop flags these)
a = %w( )
