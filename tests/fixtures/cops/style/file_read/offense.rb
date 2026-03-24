# Chain form: File.open(filename).read
File.open(filename).read
^^^^^^^^^^^^^^^^^^^^^^^^ Style/FileRead: Use `File.read`.
# Chain form with ::File prefix
::File.open(filename).read
^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/FileRead: Use `File.read`.
# Chain form with explicit 'r' mode
File.open(filename, 'r').read
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/FileRead: Use `File.read`.
# Block pass form: &:read
File.open(filename, &:read)
^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/FileRead: Use `File.read`.
# Block pass with 'r' mode
File.open(filename, 'r', &:read)
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/FileRead: Use `File.read`.
# Block form inline
File.open(filename) { |f| f.read }
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/FileRead: Use `File.read`.
# Block form multiline
File.open(filename) do |f|
^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/FileRead: Use `File.read`.
  f.read
end
# Binary mode chain
File.open(filename, 'rb').read
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/FileRead: Use `File.binread`.
# Binary mode block pass
File.open(filename, 'rb', &:read)
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/FileRead: Use `File.binread`.
# Binary mode block form
File.open(filename, 'rb') { |f| f.read }
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/FileRead: Use `File.binread`.
# r+ mode
File.open(filename, 'r+').read
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/FileRead: Use `File.read`.
# rt mode
File.open(filename, 'rt').read
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/FileRead: Use `File.read`.
# r+b mode
File.open(filename, 'r+b').read
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/FileRead: Use `File.binread`.
# r+t mode
File.open(filename, 'r+t').read
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/FileRead: Use `File.read`.
