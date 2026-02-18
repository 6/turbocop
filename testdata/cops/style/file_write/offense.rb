File.open(filename, 'w').write(content)
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/FileWrite: Use `File.write`.
File.open(filename, 'wb').write(content)
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/FileWrite: Use `File.binwrite`.
::File.open(filename, 'w').write(content)
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/FileWrite: Use `File.write`.
