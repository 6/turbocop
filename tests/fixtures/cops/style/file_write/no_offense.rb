File.write(filename, content)
File.binwrite(filename, content)
File.open(filename, 'w') do |f|
  something.write(content)
end
File.open(filename, 'r').read
File.open(filename, 'a').write(content)

# Extra keyword args (encoding:) — not a simple File.write replacement
File.open(path, 'w', encoding: Encoding::UTF_8) do |f|
  f.write(data)
end

# wb+ mode is NOT in the truncating write modes list
File.open(path, "wb+") do |f|
  f.write(data)
end
