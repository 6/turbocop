%w[foo bar]
%i[foo bar]
%W[cat dog]
%I[hello world]
%r{pattern}
%q(string)
%Q(string)
%s(symbol)
%x(command)
# percent-like text inside a string should not trigger
x = "use %w(foo bar) for arrays"
y = 'try %r{pattern} for regexp'
# percent-like text inside a comment: %i(sym1 sym2)
# content with preferred delimiters should suppress the offense
a = %r(a{3})
b = %w([some] [words])
c = %Q([string])
# interpolated content with preferred delimiters in literal parts should suppress
d = %r/a{3}#{name}/
e = %[result is (#{value})]
