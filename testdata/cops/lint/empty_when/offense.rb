case foo
when 1
^^^^ Lint/EmptyWhen: Avoid empty `when` conditions.
when 2
  do_something
end
case bar
when :a
^^^^ Lint/EmptyWhen: Avoid empty `when` conditions.
when :b
^^^^ Lint/EmptyWhen: Avoid empty `when` conditions.
when :c
  handle_c
end
