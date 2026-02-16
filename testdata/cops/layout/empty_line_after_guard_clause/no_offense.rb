def foo
  return if need_return?

  bar
end

def baz
  return if something?
  return if something_different?

  bar
end

def quux
  raise "error" unless valid?

  do_work
end

def last_guard
  return if done?
end

def consecutive_with_embedded_return
  return if does_not_expire?
  requeue! && return if not_due_yet?

  notify_remote_voters_and_owner!
end

def consecutive_mixed_guards
  raise "error" unless valid?
  do_something || return if stale?

  process
end
