IO.read("file.txt")
   ^^^^ Security/IoMethods: The use of `IO.read` is a security risk.
IO.write("file.txt", data)
   ^^^^^ Security/IoMethods: The use of `IO.write` is a security risk.
IO.binread("file.bin")
   ^^^^^^^ Security/IoMethods: The use of `IO.binread` is a security risk.
IO.binwrite("file.bin", data)
   ^^^^^^^^ Security/IoMethods: The use of `IO.binwrite` is a security risk.
IO.readlines(File.join(RAILS_ROOT, "test", "fixtures", self.class.mailer_class.name.underscore, action))
   ^^^^^^^^^ Security/IoMethods: The use of `IO.readlines` is a security risk.
included = IO.readlines file_name
              ^^^^^^^^^ Security/IoMethods: The use of `IO.readlines` is a security risk.
