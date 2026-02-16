open("| ls")
^^^^ Security/Open: The use of `Kernel#open` is a serious security risk.
open(user_input)
^^^^ Security/Open: The use of `Kernel#open` is a serious security risk.
Kernel.open(something)
       ^^^^ Security/Open: The use of `Kernel#open` is a serious security risk.
URI.open(something)
    ^^^^ Security/Open: The use of `URI.open` is a serious security risk.
open("| #{command}")
^^^^ Security/Open: The use of `Kernel#open` is a serious security risk.
