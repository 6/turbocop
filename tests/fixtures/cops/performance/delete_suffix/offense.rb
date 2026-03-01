str.gsub(/foo\z/, '')
^^^^^^^^^^^^^^^^^^^^^ Performance/DeleteSuffix: Use `delete_suffix` instead of `gsub`.
str.sub(/bar\z/, '')
^^^^^^^^^^^^^^^^^^^^ Performance/DeleteSuffix: Use `delete_suffix` instead of `sub`.
str.gsub(/suffix\z/, '')
^^^^^^^^^^^^^^^^^^^^^^^^ Performance/DeleteSuffix: Use `delete_suffix` instead of `gsub`.
str.gsub!(/suffix\z/, '')
^^^^^^^^^^^^^^^^^^^^^^^^^ Performance/DeleteSuffix: Use `delete_suffix!` instead of `gsub!`.
str.sub!(/suffix\z/, '')
^^^^^^^^^^^^^^^^^^^^^^^^ Performance/DeleteSuffix: Use `delete_suffix!` instead of `sub!`.
str.gsub(/test\.\z/, '')
^^^^^^^^^^^^^^^^^^^^^^^^ Performance/DeleteSuffix: Use `delete_suffix` instead of `gsub`.
