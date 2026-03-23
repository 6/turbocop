unless x
  do_something
end

unless condition
  foo
end

unless finished?
  retry
end

unless column_exists?(:users, :confirmed_at)
  add_column :users, :confirmed_at, :datetime
end

unless file[:directory]
  return root
end

do_something unless condition

unless a_condition
  some_method
end

something unless x.even?
