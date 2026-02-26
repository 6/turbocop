File.open("file") { |f| something }
File.open("file", &:read)
File.open("file", "w", 0o777).close
Tempfile.open("file") { |f| f.write("hi") }
StringIO.open("data")
x = 1

# Not assigned to local variable — not flagged
File.open("filename")
Tempfile.open("filename")
::File.open("filename")

# Assigned to instance/class variable — not lvasgn
@file = File.open("path")
@@file = File.open("path")

# Method chain — not lvasgn
content = File.open("path").read
data = File.open("path") { |f| f.read }

# Used as argument — not lvasgn
YAML.load(File.open("path"))
process(File.open("path"))
