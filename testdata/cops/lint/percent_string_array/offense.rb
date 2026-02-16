%w('foo' 'bar')
^^^^^^^^^^^^^^^ Lint/PercentStringArray: Within `%w`/`%W`, quotes and ',' are unnecessary and may be unwanted in the resulting strings.
%w("foo", "bar")
^^^^^^^^^^^^^^^^ Lint/PercentStringArray: Within `%w`/`%W`, quotes and ',' are unnecessary and may be unwanted in the resulting strings.
%W('baz' 'qux')
^^^^^^^^^^^^^^^^ Lint/PercentStringArray: Within `%w`/`%W`, quotes and ',' are unnecessary and may be unwanted in the resulting strings.
