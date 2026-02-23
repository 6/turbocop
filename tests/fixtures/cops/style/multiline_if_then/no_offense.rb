if cond
  a
end

if cond
elsif cond2
end

if    @io == $stdout then str << "$stdout"
elsif @io == $stdin  then str << "$stdin"
elsif @io == $stderr then str << "$stderr"
else                      str << @io.class.to_s
end

if a
  case b
  when c then
  end
end

two unless one

unless cond
  something
end
