foo ? bar : baz

bar if foo

if foo then bar end

if foo
  bar
else
  baz
end

if foo; bar; else; end

unless foo then bar else end

unless foo then x; y else z end

unless foo then (bar) else baz end

self.collect! do |x|
  if is_gap?(x) then flag = self; nil; else x; end
end

v = if opts[:exact_length] then (value.length == opts[:exact_length]) else true end

c = if chars[0][0] == ?\\ && (chars[1][0] == ?( || chars[1][0] == ?)); chars.shift; chars.shift; else; chars.shift; end

def expunged_resp
  earlier = if lpar? then label("EARLIER"); rpar; SP!; true else false end
  uids = known_uids
  data = VanishedData[uids, earlier]
end
