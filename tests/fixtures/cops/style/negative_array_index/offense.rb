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
