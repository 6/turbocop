str.delete_prefix('foo')
str.gsub(/foo/, '')
str.gsub(/\Afoo/, 'bar')
str.gsub(/\A.*foo/, '')
str.sub(/\Afo+/, '')
str.gsub(/\A@/, '')
str.gsub(/\A[a-z]+/, '')
str.gsub(/\Aprefix/i, '')
str.sub(/\Aprefix/mix, '')
url.sub(/\Ahttp:/i, "")
host.sub(/\Awww\./i, "")

# Encoding flags — RuboCop requires (regopt), encoding flags count as flags
str.sub(/\Aprefix/n, '')
str.sub(/\Aprefix/u, '')
str.gsub(/\Aprefix/e, '')
str.gsub(/\Aprefix/s, '')

# Unescaped dot is a regex metacharacter, not a literal prefix
str.sub(/\Awww./, '')
str.gsub(/\A.prefix/, '')
