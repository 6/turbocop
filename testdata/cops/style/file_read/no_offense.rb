File.read(filename)
File.binread(filename)
File.open(filename) do |f|
  something_else.read
end
something.open(filename).read
File.open(filename).write("content")
