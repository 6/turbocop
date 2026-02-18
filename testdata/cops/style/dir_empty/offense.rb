Dir.entries('path/to/dir').size == 2
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/DirEmpty: Use `Dir.empty?('path/to/dir')` instead.

Dir.children('path/to/dir').empty?
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/DirEmpty: Use `Dir.empty?('path/to/dir')` instead.

Dir.each_child('path/to/dir').none?
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/DirEmpty: Use `Dir.empty?('path/to/dir')` instead.
