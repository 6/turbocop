x.match?(/\Afoo/)
^^^^^^^^^^^^^^^^^ Performance/StartWith: Use `start_with?` instead of a regex match anchored to the beginning of the string.
str.match?(/\Abar/)
^^^^^^^^^^^^^^^^^^^ Performance/StartWith: Use `start_with?` instead of a regex match anchored to the beginning of the string.
name.match?(/\Aprefix/)
^^^^^^^^^^^^^^^^^^^^^^^ Performance/StartWith: Use `start_with?` instead of a regex match anchored to the beginning of the string.
