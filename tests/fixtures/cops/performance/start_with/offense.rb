x.match?(/\Afoo/)
^^^^^^^^^^^^^^^^^ Performance/StartWith: Use `start_with?` instead of a regex match anchored to the beginning of the string.
str.match?(/\Abar/)
^^^^^^^^^^^^^^^^^^^ Performance/StartWith: Use `start_with?` instead of a regex match anchored to the beginning of the string.
name.match?(/\Aprefix/)
^^^^^^^^^^^^^^^^^^^^^^^ Performance/StartWith: Use `start_with?` instead of a regex match anchored to the beginning of the string.
str =~ /\Aabc/
^^^^^^^^^^^^^^ Performance/StartWith: Use `start_with?` instead of a regex match anchored to the beginning of the string.
str.match(/\Aabc/)
^^^^^^^^^^^^^^^^^^ Performance/StartWith: Use `start_with?` instead of a regex match anchored to the beginning of the string.
/\Aabc/.match?(str)
^^^^^^^^^^^^^^^^^^^ Performance/StartWith: Use `start_with?` instead of a regex match anchored to the beginning of the string.
/\Aabc/.match(str)
^^^^^^^^^^^^^^^^^^ Performance/StartWith: Use `start_with?` instead of a regex match anchored to the beginning of the string.
/\Aabc/ =~ str
^^^^^^^^^^^^^^ Performance/StartWith: Use `start_with?` instead of a regex match anchored to the beginning of the string.
str.match?(/\A\.rb/)
^^^^^^^^^^^^^^^^^^^^ Performance/StartWith: Use `start_with?` instead of a regex match anchored to the beginning of the string.
str =~ /\A\#comment/
^^^^^^^^^^^^^^^^^^^^ Performance/StartWith: Use `start_with?` instead of a regex match anchored to the beginning of the string.
