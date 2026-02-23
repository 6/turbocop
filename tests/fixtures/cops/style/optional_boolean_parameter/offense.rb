def some_method(bar = false)
                ^^^^^^^^^^^ Style/OptionalBooleanParameter: Prefer keyword arguments for arguments with a boolean default value; use `bar: false` instead of `bar = false`.
end

def some_method(bar = true)
                ^^^^^^^^^^ Style/OptionalBooleanParameter: Prefer keyword arguments for arguments with a boolean default value; use `bar: true` instead of `bar = true`.
end

def some_method(foo = true, bar = 1, baz = false)
                ^^^^^^^^^^ Style/OptionalBooleanParameter: Prefer keyword arguments for arguments with a boolean default value; use `foo: true` instead of `foo = true`.
                                     ^^^^^^^^^^^ Style/OptionalBooleanParameter: Prefer keyword arguments for arguments with a boolean default value; use `baz: false` instead of `baz = false`.
end
