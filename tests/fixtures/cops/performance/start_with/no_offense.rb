x.start_with?('foo')
x.match?(/foo/)
x.match?(/\Afoo.*bar/)
x.match?(/\Afo+/)
x.match?(/\Afoo/i)
x =~ /\Afoo/i
/\Afoo/i.match?(x)
str.match?(/\A\d+/)
str =~ /\A\w+/
/\A\stest/.match?(str)
