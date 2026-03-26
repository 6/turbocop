File.zero?('path/to/file')
^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/FileEmpty: Use `File.empty?('path/to/file')` instead.
FileTest.zero?('path/to/file')
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/FileEmpty: Use `FileTest.empty?('path/to/file')` instead.
File.size('path/to/file') == 0
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/FileEmpty: Use `File.empty?('path/to/file')` instead.
File.size(report_path).zero?
^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/FileEmpty: Use `File.empty?(report_path)` instead.
File.size(@path).zero?
^^^^^^^^^^^^^^^^^^^^^^ Style/FileEmpty: Use `File.empty?(@path)` instead.
FileTest.size('path').zero?
^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/FileEmpty: Use `FileTest.empty?('path')` instead.
File.read(notes_file_path) == ""
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/FileEmpty: Use `File.empty?(notes_file_path)` instead.
File.read('path/to/file').empty?
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/FileEmpty: Use `File.empty?('path/to/file')` instead.
File.binread('path/to/file').empty?
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/FileEmpty: Use `File.empty?('path/to/file')` instead.
File.binread('path/to/file') == ''
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/FileEmpty: Use `File.empty?('path/to/file')` instead.
