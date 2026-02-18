File.open(filename, 'a') {}
^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/FileTouch: Use `FileUtils.touch(filename)` instead of `File.open` in append mode with empty block.
File.open(path, 'a') {}
^^^^^^^^^^^^^^^^^^^^^^^ Style/FileTouch: Use `FileUtils.touch(path)` instead of `File.open` in append mode with empty block.
File.open(f, 'a') {}
^^^^^^^^^^^^^^^^^^^^ Style/FileTouch: Use `FileUtils.touch(f)` instead of `File.open` in append mode with empty block.
