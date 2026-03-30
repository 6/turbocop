OpenStruct.new(name: "John")
^^^^^^^^^^ Style/OpenStructUse: Avoid using `OpenStruct`; use `Struct`, `Hash`, a class, or ActiveModel attributes instead.

x = OpenStruct.new
    ^^^^^^^^^^ Style/OpenStructUse: Avoid using `OpenStruct`; use `Struct`, `Hash`, a class, or ActiveModel attributes instead.

y = ::OpenStruct.new
    ^^^^^^^^^^^^ Style/OpenStructUse: Avoid using `OpenStruct`; use `Struct`, `Hash`, a class, or ActiveModel attributes instead.

class SubClass < OpenStruct
                 ^^^^^^^^^^ Style/OpenStructUse: Avoid using `OpenStruct`; use `Struct`, `Hash`, a class, or ActiveModel attributes instead.
end

SubClass = Class.new(OpenStruct)
                     ^^^^^^^^^^ Style/OpenStructUse: Avoid using `OpenStruct`; use `Struct`, `Hash`, a class, or ActiveModel attributes instead.

if defined?(OpenStruct::VERSION) && OpenStruct::VERSION == "0.5.2"
            ^^^^^^^^^^ Style/OpenStructUse: Avoid using `OpenStruct`; use `Struct`, `Hash`, a class, or ActiveModel attributes instead.
                                    ^^^^^^^^^^ Style/OpenStructUse: Avoid using `OpenStruct`; use `Struct`, `Hash`, a class, or ActiveModel attributes instead.
