# Block form: unless ... end with mkdir (2 offenses)
unless FileTest.exist?(path)
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Lint/NonAtomicFileOperation: Remove unnecessary existence check `FileTest.exist?`.
  FileUtils.mkdir(path)
  ^^^^^^^^^^^^^^^^^^^^^ Lint/NonAtomicFileOperation: Use atomic file operation method `FileUtils.mkdir_p`.
end

# Block form: if ... end with remove (2 offenses)
if FileTest.exist?(path)
^^^^^^^^^^^^^^^^^^^^^^^^ Lint/NonAtomicFileOperation: Remove unnecessary existence check `FileTest.exist?`.
  FileUtils.remove(path)
  ^^^^^^^^^^^^^^^^^^^^^^ Lint/NonAtomicFileOperation: Use atomic file operation method `FileUtils.rm_f`.
end

# Postfix unless (2 offenses)
FileUtils.mkdir(path) unless FileTest.exist?(path)
                      ^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Lint/NonAtomicFileOperation: Remove unnecessary existence check `FileTest.exist?`.
^^^^^^^^^^^^^^^^^^^^^ Lint/NonAtomicFileOperation: Use atomic file operation method `FileUtils.mkdir_p`.

# Postfix if (2 offenses)
FileUtils.remove(path) if FileTest.exist?(path)
                       ^^^^^^^^^^^^^^^^^^^^^^^^ Lint/NonAtomicFileOperation: Remove unnecessary existence check `FileTest.exist?`.
^^^^^^^^^^^^^^^^^^^^^^ Lint/NonAtomicFileOperation: Use atomic file operation method `FileUtils.rm_f`.

# Force method makedirs: only existence check offense (1 offense)
unless FileTest.exists?(path)
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Lint/NonAtomicFileOperation: Remove unnecessary existence check `FileTest.exists?`.
  FileUtils.makedirs(path)
end

# Force method rm_f: only existence check offense (1 offense)
if FileTest.exist?(path)
^^^^^^^^^^^^^^^^^^^^^^^^ Lint/NonAtomicFileOperation: Remove unnecessary existence check `FileTest.exist?`.
  FileUtils.rm_f(path)
end

# Force method rm_rf: only existence check offense (1 offense)
if FileTest.exist?(path)
^^^^^^^^^^^^^^^^^^^^^^^^ Lint/NonAtomicFileOperation: Remove unnecessary existence check `FileTest.exist?`.
  FileUtils.rm_rf(path)
end

# Negated if with ! (1 offense for force method)
if !FileTest.exist?(path)
^^^^^^^^^^^^^^^^^^^^^^^^^ Lint/NonAtomicFileOperation: Remove unnecessary existence check `FileTest.exist?`.
  FileUtils.makedirs(path)
end

# Dir.mkdir with Dir.exist? (2 offenses)
Dir.mkdir(path) unless Dir.exist?(path)
                ^^^^^^^^^^^^^^^^^^^^^^^ Lint/NonAtomicFileOperation: Remove unnecessary existence check `Dir.exist?`.
^^^^^^^^^^^^^^^ Lint/NonAtomicFileOperation: Use atomic file operation method `FileUtils.mkdir_p`.

# Recursive remove methods (2 offenses)
if FileTest.exist?(path)
^^^^^^^^^^^^^^^^^^^^^^^^ Lint/NonAtomicFileOperation: Remove unnecessary existence check `FileTest.exist?`.
  FileUtils.remove_entry(path)
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Lint/NonAtomicFileOperation: Use atomic file operation method `FileUtils.rm_rf`.
end

# Fully qualified constant with :: prefix on existence check
if ::FileTest.exist?(path)
^^^^^^^^^^^^^^^^^^^^^^^^^^ Lint/NonAtomicFileOperation: Remove unnecessary existence check `FileTest.exist?`.
  FileUtils.delete(path)
  ^^^^^^^^^^^^^^^^^^^^^^ Lint/NonAtomicFileOperation: Use atomic file operation method `FileUtils.rm_f`.
end

# Fully qualified constant with :: prefix on file operation
if FileTest.exist?(path)
^^^^^^^^^^^^^^^^^^^^^^^^ Lint/NonAtomicFileOperation: Remove unnecessary existence check `FileTest.exist?`.
  ::FileUtils.delete(path)
  ^^^^^^^^^^^^^^^^^^^^^^^^ Lint/NonAtomicFileOperation: Use atomic file operation method `FileUtils.rm_f`.
end

# elsif form (only existence check offense, rm_f is force method)
if condition
  do_something
elsif FileTest.exist?(path)
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Lint/NonAtomicFileOperation: Remove unnecessary existence check `FileTest.exist?`.
  FileUtils.rm_f(path)
end

# mkdir_p force method (only existence check offense)
unless FileTest.exist?(path)
^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Lint/NonAtomicFileOperation: Remove unnecessary existence check `FileTest.exist?`.
  FileUtils.mkdir_p(path)
end

# mkpath force method (only existence check offense)
unless FileTest.exist?(path)
^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Lint/NonAtomicFileOperation: Remove unnecessary existence check `FileTest.exist?`.
  FileUtils.mkpath(path)
end

# File.exist? as condition class
if File.exist?(path)
^^^^^^^^^^^^^^^^^^^^ Lint/NonAtomicFileOperation: Remove unnecessary existence check `File.exist?`.
  FileUtils.rm(path)
  ^^^^^^^^^^^^^^^^^^ Lint/NonAtomicFileOperation: Use atomic file operation method `FileUtils.rm_f`.
end

# Dir.exist? as condition class with rmdir
if Dir.exist?(path)
^^^^^^^^^^^^^^^^^^^ Lint/NonAtomicFileOperation: Remove unnecessary existence check `Dir.exist?`.
  FileUtils.rmdir(path)
  ^^^^^^^^^^^^^^^^^^^^^ Lint/NonAtomicFileOperation: Use atomic file operation method `FileUtils.rm_f`.
end

# Shell.exist? as condition class
if Shell.exist?(path)
^^^^^^^^^^^^^^^^^^^^^ Lint/NonAtomicFileOperation: Remove unnecessary existence check `Shell.exist?`.
  FileUtils.unlink(path)
  ^^^^^^^^^^^^^^^^^^^^^^ Lint/NonAtomicFileOperation: Use atomic file operation method `FileUtils.rm_f`.
end

# File.delete with File.exist? postfix if (any const receiver accepted)
File.delete(path) if File.exist?(path)
                  ^^^^^^^^^^^^^^^^^^^^ Lint/NonAtomicFileOperation: Remove unnecessary existence check `File.exist?`.
^^^^^^^^^^^^^^^^^ Lint/NonAtomicFileOperation: Use atomic file operation method `FileUtils.rm_f`.

# File.unlink with File.exist? postfix if
File.unlink(path) if File.exist?(path)
                  ^^^^^^^^^^^^^^^^^^^^ Lint/NonAtomicFileOperation: Remove unnecessary existence check `File.exist?`.
^^^^^^^^^^^^^^^^^ Lint/NonAtomicFileOperation: Use atomic file operation method `FileUtils.rm_f`.

# File.delete in block if form
if File.exist?(path)
^^^^^^^^^^^^^^^^^^^^ Lint/NonAtomicFileOperation: Remove unnecessary existence check `File.exist?`.
  File.delete(path)
  ^^^^^^^^^^^^^^^^^ Lint/NonAtomicFileOperation: Use atomic file operation method `FileUtils.rm_f`.
end

# File.unlink with space-separated args (no parens)
File.delete path if File.exist? path
                 ^^^^^^^^^^^^^^^^^^^ Lint/NonAtomicFileOperation: Remove unnecessary existence check `File.exist?`.
^^^^^^^^^^^^^^^^ Lint/NonAtomicFileOperation: Use atomic file operation method `FileUtils.rm_f`.
