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

# nested if/else among other statements should still be an offense
string.each_line do |out_line|
  line_count += 1
  if line_count > @stdout_max_lines
  ^^ Style/Next: Use `next` to skip iteration.
    out_line = "ERROR"
    if filename
      line_to_write = 1
    else
      line_to_write = 2
    end
    lines_to_write << line_to_write
  end
end

# single-statement outer unless with a sole nested if should report the inner condition
@collection.works.each do |w|
  unless w.work_facet.nil?
    if years.include?(eval(year))
    ^^ Style/Next: Use `next` to skip iteration.
      facets << w.work_facet
    end
  end
end

# single-statement outer unless with a sole nested unless should report the inner condition
cell_array.each do |cell|
  unless fields[cell.header]
    unless cell.content.blank?
    ^^^^^^ Style/Next: Use `next` to skip iteration.
      row << element
    end
  end
end

# a ternary inside the guarded body should not suppress the outer if
attributes.each do |attr, val|
  record = record.dup if record.frozen?

  if record.respond_to?("#{attr}=")
  ^^ Style/Next: Use `next` to skip iteration.
    record.attributes.key?(attr.to_s) ?
      record[attr] = val :
      record.send("#{attr}=", val)
  end
end

# multi-statement while body should report the outer unless, not the nested if
while (chunk = stdin.readpartial(opts[:sysread]))
  buf << chunk
  unless chunk.nil? || chunk.empty?
  ^^^^^^ Style/Next: Use `next` to skip iteration.
    if not opts[:quiet]
      $stdout.write(chunk)
    end
  end
end

# multi-statement block body should report the outer unless, not the nested if
CallbackRegistry.callbacks.each do |callback|
  except = callback[:options][:except]
  real_only = callback[:options][:real_requests_only]
  unless except && except.include?(options[:lib])
  ^^^^^^ Style/Next: Use `next` to skip iteration.
    if !real_only || options[:real_request]
      callback[:block].call(request_signature, response)
    end
  end
end

# multi-statement block body should keep the offense on the outer unless
collection.pages.all.each do |page|
  print "#{page.slug}\n"
  unless page.approval_delta
  ^^^^^^ Style/Next: Use `next` to skip iteration.
    if Page::COMPLETED_STATUSES.include?(page.status)
      old_transcription = if page.current_version
                            page.current_version.transcription
                          else
                            ""
                          end
      new_transcription = page.source_text
      page.update_column(:approval_delta, old_transcription.size - new_transcription.size)
    end
  end
end

# multi-statement for body should report the outer unless, not the nested unless
for item in items
  payment_item = build(item)
  unless payment_item.blank?
  ^^^^^^ Style/Next: Use `next` to skip iteration.
    unless payment_id.blank?
      update_payment_item(payment_item, payment_id)
    end
  end
end

# nested inner conditionals without else still belong to the outer unless here
all_intervals.each do |interval|
  interval_start = interval[0]
  interval_end = interval[1]
  te_date_arr = issue_entry_date_hash[entry.issue_id]
  unless te_date_arr.blank? || te_date_arr.empty?
  ^^^^^^ Style/Next: Use `next` to skip iteration.
    if te_date_arr.any? { |te_dt| te_dt.between?(interval_start, interval_end) }
      sub_quantity += get_duration(interval_start, interval_end, quantity)
    end
  end
end

# multi-statement outer unless should report itself even with nested unless
@textures.each do |texture|
  basename = check_texturename(texture.name)
  unless basename.nil?
  ^^^^^^ Style/Next: Use `next` to skip iteration.
    unless basename =~ /\.[^\.]+_atlas_.+_info_.+(_.+){6}/
      error "Texture [#{basename}] not found"
    end
  end
end
