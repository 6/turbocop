x.match?(/foo/)
^^^^^^^^^^^^^^^ Performance/StringInclude: Use `String#include?` instead of a regex match with literal-only pattern.
x.match?(/hello world/)
^^^^^^^^^^^^^^^^^^^^^^^^ Performance/StringInclude: Use `String#include?` instead of a regex match with literal-only pattern.
str.match?(/bar/)
^^^^^^^^^^^^^^^^^ Performance/StringInclude: Use `String#include?` instead of a regex match with literal-only pattern.
/foo/.match?(x)
^^^^^^^^^^^^^^^ Performance/StringInclude: Use `String#include?` instead of a regex match with literal-only pattern.
x.match(/bar/)
^^^^^^^^^^^^^^ Performance/StringInclude: Use `String#include?` instead of a regex match with literal-only pattern.
/baz/.match(x)
^^^^^^^^^^^^^^ Performance/StringInclude: Use `String#include?` instead of a regex match with literal-only pattern.
x =~ /foo/
^^^^^^^^^^ Performance/StringInclude: Use `String#include?` instead of a regex match with literal-only pattern.
/bar/ === x
^^^^^^^^^^^ Performance/StringInclude: Use `String#include?` instead of a regex match with literal-only pattern.
str !~ /abc/
^^^^^^^^^^^^ Performance/StringInclude: Use `String#include?` instead of a regex match with literal-only pattern.
/foo/ =~ str
^^^^^^^^^^^^ Performance/StringInclude: Use `String#include?` instead of a regex match with literal-only pattern.
