foo. bar
    ^ Layout/SpaceAroundMethodCallOperator: Avoid using spaces around a method call operator.

foo .bar
   ^ Layout/SpaceAroundMethodCallOperator: Avoid using spaces around a method call operator.

foo&. bar
     ^ Layout/SpaceAroundMethodCallOperator: Avoid using spaces around a method call operator.

e.  year += e._cent * 100
  ^^ Layout/SpaceAroundMethodCallOperator: Avoid using spaces around a method call operator.

Unset. ("params" => {decide: true}).inspect("a", "b", "c", "d", "e").must_equal %{<Result:true [false, true, nil, 1, 2] >}
      ^ Layout/SpaceAroundMethodCallOperator: Avoid using spaces around a method call operator.
