set_app("RuboCop")
website  = "https://github.com/rubocop"
       ^ Layout/ExtraSpacing: Unnecessary spacing detected.

x  = 1
 ^ Layout/ExtraSpacing: Unnecessary spacing detected.

method_call(arg1,  arg2)
                 ^ Layout/ExtraSpacing: Unnecessary spacing detected.

# Alignment where adjacent token is NOT preceded by space (coincidental vertical alignment)
d_is_vertically_aligned  do
                       ^ Layout/ExtraSpacing: Unnecessary spacing detected.
  _______________________d
end

# Extra space before a float in multiline array
{:a => "a",
 :b => [nil,  2.5]}
            ^ Layout/ExtraSpacing: Unnecessary spacing detected.

# Extra spacing in class inheritance
class A   < String
       ^^ Layout/ExtraSpacing: Unnecessary spacing detected.
end

# Extra spacing before a unary plus in an argument list
assert_difference(MyModel.count, +2,
                  3,  +3,
                    ^ Layout/ExtraSpacing: Unnecessary spacing detected.
                  4,+4)

# Single-line hash with extra spaces
hash = {a:   1,  b:    2}
          ^^ Layout/ExtraSpacing: Unnecessary spacing detected.
               ^ Layout/ExtraSpacing: Unnecessary spacing detected.
                   ^^^ Layout/ExtraSpacing: Unnecessary spacing detected.

# Trailing comments at different columns - NOT aligned, should be flagged
check_a_pattern_result   # comment A
                      ^ Layout/ExtraSpacing: Unnecessary spacing detected.
check_b   # comment B
       ^ Layout/ExtraSpacing: Unnecessary spacing detected.
check_c_patterns   # comment C
                ^ Layout/ExtraSpacing: Unnecessary spacing detected.

# Extra spaces inside empty word arrays (RuboCop flags these)
a = %w(  )
       ^ Layout/ExtraSpacing: Unnecessary spacing detected.
