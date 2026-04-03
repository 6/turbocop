[].each do |o|
  next unless o == 1
  puts o
end
[].each do |o|
  if o == 1
    puts o
  else
    puts "other"
  end
end
[].each { |a| return 'yes' if a == 1 }
items.map do |item|
  if item
    process(item)
  end
end

items.flat_map do |item|
  if item
    [process(item)]
  end
end

rows.each do |row|
  if col_len > max_len
    if col_len > MAX_COL_WIDTH
      max[idx] = MAX_COL_WIDTH
    else
      max[idx] = col_len
    end
  end
end

rows.each do |row|
  if outer_cond
    unless inner_cond
      warn row
    else
      work row
      work row
      work row
    end
  end
end
