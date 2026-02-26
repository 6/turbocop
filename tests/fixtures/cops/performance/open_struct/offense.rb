OpenStruct.new(name: "test")
^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Performance/OpenStruct: Use `Struct` instead of `OpenStruct`.
x = OpenStruct.new
    ^^^^^^^^^^^^^^ Performance/OpenStruct: Use `Struct` instead of `OpenStruct`.
OpenStruct.new(foo: 1, bar: 2)
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Performance/OpenStruct: Use `Struct` instead of `OpenStruct`.
::OpenStruct.new(name: "test")
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Performance/OpenStruct: Use `Struct` instead of `OpenStruct`.
