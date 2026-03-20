open("| ls")
^^^^ Security/Open: The use of `Kernel#open` is a serious security risk.
open(user_input)
^^^^ Security/Open: The use of `Kernel#open` is a serious security risk.
URI.open(something)
    ^^^^ Security/Open: The use of `URI.open` is a serious security risk.
URI.open(user_input) # standard:disable Security/Open
    ^^^^ Security/Open: The use of `URI.open` is a serious security risk.
::URI.open(something)
      ^^^^ Security/Open: The use of `::URI.open` is a serious security risk.
open("| #{command}")
^^^^ Security/Open: The use of `Kernel#open` is a serious security risk.
open(&block)
^^^^ Security/Open: The use of `Kernel#open` is a serious security risk.
