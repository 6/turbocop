def func()
        ^^ Style/DefWithParentheses: Omit the parentheses in defs when the method doesn't accept any arguments.
end

def Test.func()
             ^^ Style/DefWithParentheses: Omit the parentheses in defs when the method doesn't accept any arguments.
  something
end

def bar()
       ^^ Style/DefWithParentheses: Omit the parentheses in defs when the method doesn't accept any arguments.
  42
end
