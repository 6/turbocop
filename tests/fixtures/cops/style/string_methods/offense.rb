'foo'.intern
      ^^^^^^ Style/StringMethods: Prefer `to_sym` over `intern`.

x.intern
  ^^^^^^ Style/StringMethods: Prefer `to_sym` over `intern`.

name.intern
     ^^^^^^ Style/StringMethods: Prefer `to_sym` over `intern`.

id1 = intern :foo
      ^^^^^^ Style/StringMethods: Prefer `to_sym` over `intern`.

id2 = intern :foo
      ^^^^^^ Style/StringMethods: Prefer `to_sym` over `intern`.

id1 = intern :id1
      ^^^^^^ Style/StringMethods: Prefer `to_sym` over `intern`.

id2 = intern :id1
      ^^^^^^ Style/StringMethods: Prefer `to_sym` over `intern`.

id3 = intern :id3
      ^^^^^^ Style/StringMethods: Prefer `to_sym` over `intern`.

intern(:test).inspect.should == "test"
^^^^^^ Style/StringMethods: Prefer `to_sym` over `intern`.

intern(:true).inspect.should == "#|true|"
^^^^^^ Style/StringMethods: Prefer `to_sym` over `intern`.

intern(:false).inspect.should == "#|false|"
^^^^^^ Style/StringMethods: Prefer `to_sym` over `intern`.

task = intern(task_class, task_name)
       ^^^^^^ Style/StringMethods: Prefer `to_sym` over `intern`.
