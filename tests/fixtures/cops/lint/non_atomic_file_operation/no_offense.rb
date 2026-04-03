# Standalone file operations (no existence check)
FileUtils.mkdir_p(path)
FileUtils.rm_f(path)
FileUtils.rm_rf(path)
FileUtils.makedirs(path)
Dir.exist?(path)
File.exist?(path)

# Checking existence of different path than operation
FileUtils.mkdir_p(y) unless FileTest.exist?(path)

# Not a file operation method
unless FileUtils.exist?(path)
  FileUtils.options_of(:rm)
end

# Not a recognized receiver
unless FileUtils.exist?(path)
  NotFile.remove(path)
end

# Not an exist check method
unless FileUtils.options_of(:rm)
  FileUtils.mkdir_p(path)
end

if FileTest.executable?(path)
  FileUtils.remove(path)
end

# Multiple statements in body (not just file op)
unless FileTest.exist?(path)
  FileUtils.makedirs(path)
  do_something
end

unless FileTest.exist?(path)
  do_something
  FileUtils.makedirs(path)
end

# If with else branch
if FileTest.exist?(path)
  FileUtils.mkdir(path)
else
  do_something
end

# Complex conditional with &&
if FileTest.exist?(path) && File.stat(path).socket?
  FileUtils.mkdir(path)
end

FileUtils.mkdir(path) if FileTest.exist?(path) && File.stat(path).socket?

# Complex conditional with ||
if FileTest.exist?(path) || condition
  FileUtils.mkdir(path)
end

# No explicit receiver on file operation
mkdir(path) unless FileTest.exist?(path)

# Non-constant receiver
storage[:files].delete(file) unless File.exists?(file)

# force: false explicitly set (not an offense)
unless FileTest.exists?(path)
  FileUtils.makedirs(path, force: false)
end

# Space before paren causes argument mismatch (RuboCop compares AST nodes;
# the exist? argument is wrapped in a begin/paren node, the file op is not)
if Dir.exist? (path)
  FileUtils.rm_rf(path)
end

# rm_r and rmtree are not recognized methods
if FileTest.exist?(path)
  FileUtils.rm_r(path)
end

if FileTest.exist?(path)
  FileUtils.rmtree(path)
end
