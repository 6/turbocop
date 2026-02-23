OpenStruct.new(name: "John")
^^^^^^^^^^ Style/OpenStructUse: Avoid using `OpenStruct`; use `Struct`, `Hash`, a class, or ActiveModel attributes instead.

x = OpenStruct.new
    ^^^^^^^^^^ Style/OpenStructUse: Avoid using `OpenStruct`; use `Struct`, `Hash`, a class, or ActiveModel attributes instead.

y = ::OpenStruct.new
    ^^^^^^^^^^^^ Style/OpenStructUse: Avoid using `OpenStruct`; use `Struct`, `Hash`, a class, or ActiveModel attributes instead.
