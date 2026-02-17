items.each { |x| puts x }

items.each do |x|
  puts x
end

foo { bar }

# Block with only a comment (AllowComments default true)
items.each do |x|
  # TODO: implement
end

# Inline empty block with trailing comment on same line
Mail.connection {} # rubocop:disable:block
foo.bar {} # some explanation comment
