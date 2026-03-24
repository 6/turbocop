variable.to_sym
:foo.to_s
x.to_sym
name = 'foo'
name.to_sym
result = :bar
{ normal: 'val' }
{ another_key: 1, foo: 2 }
{ 'has-hyphen': 1 }
{ 'has space': 1 }
{ "7_days": 1 }
:'foo-bar'
:"foo-bar"
:'Foo/Bar/Baz'
:'foo-bar""'
:normal
{ '==': 'bar' }
{ 'foo:bar': 'bar' }
{ 'foo=': 'bar' }
to_sym == other
%i(foo bar)
alias foo bar
{ foo: :bar }
# Symbol with escape sequences that need quotes
:"\n"
:"\t"
:"foo\nbar"
# Empty symbol
:""
# to_sym on variable
name.to_sym
# method call that looks like to_sym but has args
"foo".to_sym(1)
# Chained method call
"foo".upcase.to_sym
# Rocket-style hash keys with non-identifier-start values
# RuboCop skips these in correct_hash_key (/\A[a-z0-9_]/i fails)
{ :'@ivar' => 1 }
{ :"@ivar" => 1 }
{ :'$global' => 1 }
{ :'+' => 1 }
{ :'==' => 1 }
{ :'@@cvar' => 1 }
# Setter-like operator symbols (ends with =) are left alone
:"!="
:"=="
# Alias arguments — quoted symbols in alias are not flaggable
# because a symbol requiring quotes is not a valid method identifier
alias :'foo' bar
alias :"foo" bar
alias foo :'bar'
alias foo :"bar"
alias :'foo' :'bar'
alias :"foo" :"bar"
# Symbols with non-printable escape sequences — quoting IS necessary
:"\x00"
:"\x01"
:"\x1F"
:"\x7F"
:"\a"
:"\b"
:"\e"
# Symbol with escaped interpolation guard — quoting IS necessary
:"\#{c}"
:"\#{foo}"
:"\#@ivar"
:"\#$global"
# UTF-8 symbols that still require quoting (contain non-identifier chars)
:"foo-café"
:"résumé bar"
# Bare UTF-8 symbols (already unquoted, no offense)
:résumé
:café
# Emoji hash keys cannot be bare symbols — emoji are not valid Ruby identifiers
{ "🇺🇸": "hello" }
{ "🏠": "house" }
{ "🎉": "party" }
# Non-ASCII colon-style hash keys — RuboCop skips these because /\A[a-z0-9_]/i fails
{ 'æ': 'ae' }
{ 'ó': 'o' }
{ 'ł': 'l' }
{ "а": "a" }
{ "Ãa1": "true" }
# %s() percent-literal symbols with non-quote delimiters — RuboCop never flags these
%s(CourierNew)
%s(CourierNew,Italic)
%s[bar]
