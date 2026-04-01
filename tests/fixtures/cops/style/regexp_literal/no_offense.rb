foo = /a/
bar = /hello world/
baz = /\d+/
x = /foo/i
y = /test/
z = 'hello'
# %r with space-starting content avoids syntax error as method arg
do_something %r{ regexp}
foo&.do_something %r{ regexp}
str.gsub(%r{ rubocop}, ',')
str.match(%r{=foo})
# %r with inner slashes is always fine (even in 'slashes' style)
%r{foo/bar}
/foo/
# Slashes inside interpolation should not count as inner slashes
/#{Regexp.quote(">" + content + "</")}/
/#{path + "/" + file}/
/#{a}/
/prefix#{"/middle"}/
