eval(user_input)
^^^^ Security/Eval: The use of `eval` is a serious security risk.
eval(user_input) # standard:disable Security/Eval
^^^^ Security/Eval: The use of `eval` is a serious security risk.
Kernel.eval(code_var)
       ^^^^ Security/Eval: The use of `eval` is a serious security risk.
binding.eval(dynamic_code)
        ^^^^ Security/Eval: The use of `eval` is a serious security risk.
