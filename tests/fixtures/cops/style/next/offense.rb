[].each do |o|
  if o == 1
  ^^ Style/Next: Use `next` to skip iteration.
    puts o
    puts o
    puts o
  end
end

3.downto(1) do
  if true
  ^^ Style/Next: Use `next` to skip iteration.
    a = 1
    b = 2
    c = 3
  end
end

items.map do |item|
  unless item.nil?
  ^^^^^^ Style/Next: Use `next` to skip iteration.
    process(item)
    transform(item)
    finalize(item)
  end
end

# Last statement in multi-statement block body
[].each do |o|
  x = 1
  if o == 1
  ^^ Style/Next: Use `next` to skip iteration.
    puts o
    puts o
    puts o
  end
end

# for loop with if/unless as sole body
for post in items
  unless post.nil?
  ^^^^^^ Style/Next: Use `next` to skip iteration.
    process(post)
    transform(post)
    finalize(post)
  end
end

# for loop with last-statement pattern
for item in items
  x = process(item)
  if item.valid?
  ^^ Style/Next: Use `next` to skip iteration.
    transform(item)
    save(item)
    finalize(item)
  end
end

# while loop
while running
  if test
  ^^ Style/Next: Use `next` to skip iteration.
    something
    something
    something
  end
end

# until loop
until finished
  if test
  ^^ Style/Next: Use `next` to skip iteration.
    something
    something
    something
  end
end

# loop method
loop do
  if test
  ^^ Style/Next: Use `next` to skip iteration.
    something
    something
    something
  end
end

# multiline single-statement body still counts toward MinBodyLength
for post in @posts
  unless post.user.is_spammer?
  ^^^^^^ Style/Next: Use `next` to skip iteration.
    xml.item do
      xml.title post.title
      xml.description markdown(post.text)
      xml.pubDate post.created_at.to_s(:rfc822)
      xml.link post_url(post)
      xml.comments post_url(post)
      xml.guid post_url(post)
    end
  end
end

# multiline nested block body with only one top-level statement
items.each do |item|
  if condition
  ^^ Style/Next: Use `next` to skip iteration.
    do_work do
      step_one(item)
      step_two(item)
      step_three(item)
    end
  end
end

# body line span matters even when there are only two top-level statements
response.each do |k, v|
  next unless v.is_a?(Hash) && k != :suggested_template_model

  response[k] = HashHelper.to_ruby(v)

  if response[k].has_key?(:validation_errors)
  ^^ Style/Next: Use `next` to skip iteration.
    ruby_hashes = response[k][:validation_errors].map do |err|
      HashHelper.to_ruby(err)
    end
    response[k][:validation_errors] = ruby_hashes
  end
end

# multiline hash literal body should not be measured by statement count
@blocks.each_with_index.map do |row_blocks, row_index|
  column_block_with_column_index = row_blocks.each_with_index.to_a.reverse.detect do |column_block, column_index|
    !column_block.clear?
  end
  if column_block_with_column_index
  ^^ Style/Next: Use `next` to skip iteration.
    right_most_block = column_block_with_column_index[0]
    {
      block: right_most_block,
      row_index: row_index,
      column_index: column_block_with_column_index[1]
    }
  end
end

if component[0] == EXCLUDE
^ Style/Next: Use `next` to skip iteration.

if bytes_written > size && !warned
^ Style/Next: Use `next` to skip iteration.

if line_count > @stdout_max_lines
^ Style/Next: Use `next` to skip iteration.

if item['name'] == item_name
^ Style/Next: Use `next` to skip iteration.

if item['name'] == item_name
^ Style/Next: Use `next` to skip iteration.

if handshake_packet.identify?(packet.buffer(false))
^ Style/Next: Use `next` to skip iteration.

if time_difference <= 0 && @share.snooze_base.not_queued?(reaction: reaction)
^ Style/Next: Use `next` to skip iteration.

if cmd_array
^ Style/Next: Use `next` to skip iteration.
