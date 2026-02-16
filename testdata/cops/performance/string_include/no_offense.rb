x.include?('foo')
x.match?(/foo.*bar/)
x.match?(/\Afoo/)
x.match?(/foo\z/)
x.match?(/fo+o/)
# Regex with flags — can't use include?
x.match?(/pattern/i)
x.match?(/literal/im)
