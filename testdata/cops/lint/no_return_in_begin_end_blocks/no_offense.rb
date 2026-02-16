@some_variable ||= begin
  if some_condition_is_met
    some_value
  else
    do_something
  end
end

x = if condition
  return 1
end
