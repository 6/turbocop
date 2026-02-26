case foo
when 1
  bar
when 2
  baz
end
case foo
when 4
  foobar
else
  baz
end
case foo
when 1
  bar
else
  baz
end
x = 42
case foo
when 4
  foobar
when *cond
  bar
else
  baz
end
case foo
when 4
  foobar
when *cond1
  bar
when *cond2
  doo
else
  baz
end
case foo
when *[1, 2]
  bar
when *[3, 4]
  bar
when 5
  baz
end
case foo
when *[1, 2]
  bar
end
case foo
when *cond1
  bar
when *cond2
  doo
when *cond3
  baz
end
case action
when 'store'
  data
when *VOIDABLE_ACTIONS
  action
else
  data
end
case event_type
when *ORDER_EVENTS, *ACCOUNT_EVENTS
  handle_event
else
  handle_legacy
end
case foo
when *ITEMS.map(&:to_s)
  bar
end
