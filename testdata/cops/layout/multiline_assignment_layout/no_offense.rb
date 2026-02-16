blarg =
  if true
    'yes'
  else
    'no'
  end

result =
  case x
  when :a
    1
  else
    2
  end

value =
  begin
    compute
  rescue => e
    nil
  end

x = 42
