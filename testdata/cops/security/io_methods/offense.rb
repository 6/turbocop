IO.read("file.txt")
   ^^^^ Security/IoMethods: The use of `IO.read` is a security risk.
IO.write("file.txt", data)
   ^^^^^ Security/IoMethods: The use of `IO.write` is a security risk.
IO.binread("file.bin")
   ^^^^^^^ Security/IoMethods: The use of `IO.binread` is a security risk.
