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
str =~ /1\.9/
^^^^^^^^^^^^^ Performance/StringInclude: Use `String#include?` instead of a regex match with literal-only pattern.
str =~ /common\.rb/
^^^^^^^^^^^^^^^^^^^ Performance/StringInclude: Use `String#include?` instead of a regex match with literal-only pattern.
/\t/.match(str)
^^^^^^^^^^^^^^^ Performance/StringInclude: Use `String#include?` instead of a regex match with literal-only pattern.
/\n/.match(str)
^^^^^^^^^^^^^^^ Performance/StringInclude: Use `String#include?` instead of a regex match with literal-only pattern.
str.match?(/\$DEBUG/)
^^^^^^^^^^^^^^^^^^^^^ Performance/StringInclude: Use `String#include?` instead of a regex match with literal-only pattern.
str&.match?(/abc/)
^^^^^^^^^^^^^^^^^^ Performance/StringInclude: Use `String#include?` instead of a regex match with literal-only pattern.
str =~ /\#bar/
^^^^^^^^^^^^^^ Performance/StringInclude: Use `String#include?` instead of a regex match with literal-only pattern.
/\\/.match?(str)
^^^^^^^^^^^^^^^^ Performance/StringInclude: Use `String#include?` instead of a regex match with literal-only pattern.
