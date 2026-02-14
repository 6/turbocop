x.match?(/foo/)
^^^^^^^^^^^^^^^ Performance/StringInclude: Use `String#include?` instead of a regex match with literal-only pattern.
x.match?(/hello world/)
^^^^^^^^^^^^^^^^^^^^^^^^ Performance/StringInclude: Use `String#include?` instead of a regex match with literal-only pattern.
str.match?(/bar/)
^^^^^^^^^^^^^^^^^ Performance/StringInclude: Use `String#include?` instead of a regex match with literal-only pattern.
