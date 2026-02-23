File.zero?('path/to/file')
^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/FileEmpty: Use `File.empty?('path/to/file')` instead.
FileTest.zero?('path/to/file')
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/FileEmpty: Use `FileTest.empty?('path/to/file')` instead.
File.size('path/to/file') == 0
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/FileEmpty: Use `File.empty?('path/to/file')` instead.
