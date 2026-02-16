open("| ls")
^^^^ Security/Open: The use of `Kernel#open` is a serious security risk.
open(user_input)
^^^^ Security/Open: The use of `Kernel#open` is a serious security risk.
Kernel.open("file")
       ^^^^ Security/Open: The use of `Kernel#open` is a serious security risk.
