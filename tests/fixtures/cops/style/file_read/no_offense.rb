# Already using File.read
File.read(filename)
# Already using File.binread
File.binread(filename)
# Block reads something other than the block variable
File.open(filename) do |f|
  something_else.read
end
# Not a File class
something.open(filename).read
# Not .read method
File.open(filename).write("content")
# .read with arguments (length/offset) - not a simple read-all
File.open(filename).read(100)
# Write mode - not a read mode
File.open(filename, 'w').read
# Append mode
File.open(filename, 'a').read
# Block form reading something else
File.open(filename) { |f| other.read }
# Block form with extra statements
File.open(filename) do |f|
  data = f.read
end
