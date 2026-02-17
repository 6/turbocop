alive_threads = Thread.list.select do |t|
  t.alive?
end
alive_threads.map do |t|
  t.object_id
end

items.select { |i| i.valid? }.map { |i| i.name }

foo.each { |x| x }.count
