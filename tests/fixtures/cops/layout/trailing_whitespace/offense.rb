x = 1   
     ^^^ Layout/TrailingWhitespace: Trailing whitespace detected.
y = 2 
     ^ Layout/TrailingWhitespace: Trailing whitespace detected.
z = 3
a = 4  
     ^^ Layout/TrailingWhitespace: Trailing whitespace detected.
wide = :a　
         ^ Layout/TrailingWhitespace: Trailing whitespace detected.
# comment with trailing spaces 
                              ^ Layout/TrailingWhitespace: Trailing whitespace detected.
# Required  Numbers  Only 
                         ^ Layout/TrailingWhitespace: Trailing whitespace detected.
#     
 ^^^^^ Layout/TrailingWhitespace: Trailing whitespace detected.
nbsp = :a 
         ^ Layout/TrailingWhitespace: Trailing whitespace detected.
# note 
      ^ Layout/TrailingWhitespace: Trailing whitespace detected.

file = StringIO.new <<-RUBY
__END__
RUBY
after = 1 
         ^ Layout/TrailingWhitespace: Trailing whitespace detected.
