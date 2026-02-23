x.match?(/foo\z/)
^^^^^^^^^^^^^^^^^ Performance/EndWith: Use `end_with?` instead of a regex match anchored to the end of the string.
str.match?(/bar\z/)
^^^^^^^^^^^^^^^^^^^ Performance/EndWith: Use `end_with?` instead of a regex match anchored to the end of the string.
name.match?(/rb\z/)
^^^^^^^^^^^^^^^^^^^ Performance/EndWith: Use `end_with?` instead of a regex match anchored to the end of the string.
