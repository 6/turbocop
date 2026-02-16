class A
  @@test = 10
  ^^^^^^ Style/ClassVars: Replace class var @@test with a class instance var.
end

class B
  @@count = 0
  ^^^^^^^ Style/ClassVars: Replace class var @@count with a class instance var.
end

class C
  @@name = "test"
  ^^^^^^ Style/ClassVars: Replace class var @@name with a class instance var.
end
