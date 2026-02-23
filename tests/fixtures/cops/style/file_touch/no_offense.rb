FileUtils.touch(filename)
File.open(filename, 'a')
File.open(filename, 'w') {}
File.open(filename) {}
File.open(filename, 'a') { |f| f.write("x") }
