items.each do |x|
  puts x
end
[1, 2].map do |x|
  x * 2
end
foo.select do |x|
  x > 1
end
make_routes = -> (a) {
  a.map { |c| c.name }
}
action = -> () {
  do_something
}
handler = -> (opts = {}) {
  opts.reduce({}) do |memo, k|
    memo
  end
}
