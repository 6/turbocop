str.gsub(/\Afoo/, '')
^^^^^^^^^^^^^^^^^^^^^ Performance/DeletePrefix: Use `delete_prefix` instead of `gsub`.
str.sub(/\Abar/, '')
^^^^^^^^^^^^^^^^^^^^ Performance/DeletePrefix: Use `delete_prefix` instead of `sub`.
str.gsub(/\Aprefix/, '')
^^^^^^^^^^^^^^^^^^^^^^^^ Performance/DeletePrefix: Use `delete_prefix` instead of `gsub`.
str.gsub!(/\Aprefix/, '')
^^^^^^^^^^^^^^^^^^^^^^^^^ Performance/DeletePrefix: Use `delete_prefix!` instead of `gsub!`.
str.sub!(/\Aprefix/, '')
^^^^^^^^^^^^^^^^^^^^^^^^ Performance/DeletePrefix: Use `delete_prefix!` instead of `sub!`.
