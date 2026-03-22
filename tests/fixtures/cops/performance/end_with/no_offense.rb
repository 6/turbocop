x.end_with?('foo')
x.match?(/foo/)
x.match?(/foo.*\z/)
x.match?(/foo+\z/)
x =~ /foo\z/i
x.match?(/\w\z/)
x.match?(/\d\z/)
x.match?(/\s\z/)

# Encoding flags — RuboCop requires (regopt), encoding flags count as flags
str.match?(/suffix\z/n)
str =~ /suffix\z/u
/suffix\z/e.match?(str)
/suffix\z/s =~ str
