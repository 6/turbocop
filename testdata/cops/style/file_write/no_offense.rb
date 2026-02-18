File.write(filename, content)
File.binwrite(filename, content)
File.open(filename, 'w') do |f|
  something.write(content)
end
File.open(filename, 'r').read
File.open(filename, 'a').write(content)
