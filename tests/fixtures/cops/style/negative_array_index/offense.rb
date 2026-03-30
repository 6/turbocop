arr[arr.length - 1]
    ^^^^^^^^^^^^^^ Style/NegativeArrayIndex: Use `arr[-1]` instead of `arr[arr.length - 1]`.

arr[arr.size - 2]
    ^^^^^^^^^^^^ Style/NegativeArrayIndex: Use `arr[-2]` instead of `arr[arr.size - 2]`.

foo[foo.length - 3]
    ^^^^^^^^^^^^^^ Style/NegativeArrayIndex: Use `foo[-3]` instead of `foo[foo.length - 3]`.

arr[arr.count - 4]
    ^^^^^^^^^^^^^^ Style/NegativeArrayIndex: Use `arr[-4]` instead of `arr[arr.count - 4]`.

@arr[@arr.length - 2]
     ^^^^^^^^^^^^^^^^ Style/NegativeArrayIndex: Use `@arr[-2]` instead of `@arr[@arr.length - 2]`.

CONST[CONST.size - 1]
      ^^^^^^^^^^^^^^^ Style/NegativeArrayIndex: Use `CONST[-1]` instead of `CONST[CONST.size - 1]`.

arr.sort[arr.sort.length - 2]
         ^^^^^^^^^^^^^^^^^^^^ Style/NegativeArrayIndex: Use `arr.sort[-2]` instead of `arr.sort[arr.sort.length - 2]`.

arr.sort[arr.length - 2]
         ^^^^^^^^^^^^^^^ Style/NegativeArrayIndex: Use `arr.sort[-2]` instead of `arr.sort[arr.length - 2]`.

arr.sort[arr.reverse.length - 2]
         ^^^^^^^^^^^^^^^^^^^^^^^ Style/NegativeArrayIndex: Use `arr.sort[-2]` instead of `arr.sort[arr.reverse.length - 2]`.

arr[(0..(arr.length - 2))]
        ^^^^^^^^^^^^^^^^ Style/NegativeArrayIndex: Use `arr[(0..-2)]` instead of `arr[(0..(arr.length - 2))]`.

arr[(0...(arr.length - 4))]
         ^^^^^^^^^^^^^^^^ Style/NegativeArrayIndex: Use `arr[(0...-4)]` instead of `arr[(0...(arr.length - 4))]`.

arr[(1..(arr.size - 2))]
        ^^^^^^^^^^^^^^^ Style/NegativeArrayIndex: Use `arr[(1..-2)]` instead of `arr[(1..(arr.size - 2))]`.

@headline_number_stack[@headline_number_stack.length - 1] += 1
                       ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/NegativeArrayIndex: Use `@headline_number_stack[-1]` instead of `@headline_number_stack[@headline_number_stack.length - 1]`.

"#{token[0,1]}***#{token[token.length-1,1]}"
                         ^^^^^^^^^^^^^^ Style/NegativeArrayIndex: Use `token[-1]` instead of `token[token.length-1]`.

(c[c.size-1,1]=='%') ? (c.to_f*2.55).round : c.to_i
   ^^^^^^^^ Style/NegativeArrayIndex: Use `c[-1]` instead of `c[c.size-1]`.

if line[0, 1] == "[" and line[line.length - 1, 1] == "]"
                              ^^^^^^^^^^^^^^^ Style/NegativeArrayIndex: Use `line[-1]` instead of `line[line.length - 1]`.

add_sentence_internal(sentence[0, sentence.length - 1], sentence[sentence.length - 1, 1])
                                                                 ^^^^^^^^^^^^^^^^^^^ Style/NegativeArrayIndex: Use `sentence[-1]` instead of `sentence[sentence.length - 1]`.

return unless message[message.length - 1, 1] == "?"
                      ^^^^^^^^^^^^^^^^^^ Style/NegativeArrayIndex: Use `message[-1]` instead of `message[message.length - 1]`.

elsif token[token.size - 1, 1] =~ /[!?]/
            ^^^^^^^^^^^^^^ Style/NegativeArrayIndex: Use `token[-1]` instead of `token[token.size - 1]`.

if value.size > 4 && value[0, 2] == "\\\"" && value[value.size - 2, value.size] == "\\\""
                                                    ^^^^^^^^^^^^^^ Style/NegativeArrayIndex: Use `value[-2]` instead of `value[value.size - 2]`.
