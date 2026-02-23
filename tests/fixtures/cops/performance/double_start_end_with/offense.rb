x.start_with?("a") || x.start_with?("b")
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Performance/DoubleStartEndWith: Use `start_with?` with multiple arguments instead of chaining `||`.
x.end_with?(".rb") || x.end_with?(".py")
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Performance/DoubleStartEndWith: Use `end_with?` with multiple arguments instead of chaining `||`.
str.start_with?("http") || str.start_with?("ftp")
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Performance/DoubleStartEndWith: Use `start_with?` with multiple arguments instead of chaining `||`.
