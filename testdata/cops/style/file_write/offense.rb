File.open(filename, 'w').write(content)
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/FileWrite: Use `File.write`.
File.open(filename, 'wb').write(content)
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/FileWrite: Use `File.binwrite`.
::File.open(filename, 'w').write(content)
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/FileWrite: Use `File.write`.
File.open(filename, 'w') { |f| f.write(content) }
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/FileWrite: Use `File.write`.
File.open(filename, 'wb') { |io| io.write(content) }
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/FileWrite: Use `File.binwrite`.
File.open(filename, 'w') do |f|
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/FileWrite: Use `File.write`.
  f.write(content)
end
