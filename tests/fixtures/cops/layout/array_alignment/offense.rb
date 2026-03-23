x = [1,
  2,
  ^ Layout/ArrayAlignment: Align the elements of an array literal if they span more than one line.
  3]
  ^ Layout/ArrayAlignment: Align the elements of an array literal if they span more than one line.
y = [:a,
       :b,
       ^^ Layout/ArrayAlignment: Align the elements of an array literal if they span more than one line.
       :c]
       ^^ Layout/ArrayAlignment: Align the elements of an array literal if they span more than one line.
z = ["x",
         "y"]
         ^^ Layout/ArrayAlignment: Align the elements of an array literal if they span more than one line.

# Trailing comma creates implicit array — misaligned elements
config[:expiration] = valid_date,
config[:key_name] = key_name
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Layout/ArrayAlignment: Align the elements of an array literal if they span more than one line.

t[:push] = "Commit changes",
t[:pull] = "Update working copy",
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Layout/ArrayAlignment: Align the elements of an array literal if they span more than one line.
t[:switch] = "Open branch"
^^^^^^^^^^^^^^^^^^^^^^^^^^ Layout/ArrayAlignment: Align the elements of an array literal if they span more than one line.

MAX_LENGTH = "x-max-length",
QUEUE_TYPE = "x-queue-type"
^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Layout/ArrayAlignment: Align the elements of an array literal if they span more than one line.

# Array inside if/else within multi-assignment — arrays nested in
# control flow are NOT direct children of masgn, so should be checked
name, size = if condition
  ["first",
    target_size,
    ^ Layout/ArrayAlignment: Align the elements of an array literal if they span more than one line.
    target_storage]
    ^ Layout/ArrayAlignment: Align the elements of an array literal if they span more than one line.
else
  ["second",
    other_size,
    ^ Layout/ArrayAlignment: Align the elements of an array literal if they span more than one line.
    other_storage]
    ^ Layout/ArrayAlignment: Align the elements of an array literal if they span more than one line.
end

# Bracketed array inside multi-assignment with multiple RHS values —
# the array's parent is the implicit RHS array, not the masgn itself
res, ignored = [items.select { |f| !File.directory?(f) },
items.select { |f| File.directory?(f) }], Dir.glob(".*")
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Layout/ArrayAlignment: Align the elements of an array literal if they span more than one line.

# Rescue exception list misaligned
begin
  foo
rescue ArgumentError,
  RuntimeError,
  ^^^^^^^^^^^^^^^^^^ Layout/ArrayAlignment: Align the elements of an array literal if they span more than one line.
  TypeError => e
  ^^^^^^^^^^^^ Layout/ArrayAlignment: Align the elements of an array literal if they span more than one line.
  bar
end
