str.delete_suffix('foo')
str.gsub(/foo/, '')
str.gsub(/foo\z/, 'bar')
str.gsub(/.*foo\z/, '')
str.sub(/fo+\z/, '')
# Regex with flags — can't use delete_suffix
str.gsub(/suffix\z/i, '')
str.sub(/suffix\z/m, '')
