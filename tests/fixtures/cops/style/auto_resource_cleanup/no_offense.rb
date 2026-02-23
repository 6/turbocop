File.open("file") { |f| something }
File.open("file", &:read)
File.open("file", "w", 0o777).close
Tempfile.open("file") { |f| f.write("hi") }
StringIO.open("data")
x = 1
