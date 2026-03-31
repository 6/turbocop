puts(compute something)
     ^^^^^^^^^^^^^^^^^ Style/NestedParenthesizedCalls: Add parentheses to nested method call `compute something`.

method1(method2 a, b)
        ^^^^^^^^^^^^ Style/NestedParenthesizedCalls: Add parentheses to nested method call `method2 a, b`.

foo(bar baz)
    ^^^^^^^ Style/NestedParenthesizedCalls: Add parentheses to nested method call `bar baz`.

expect(links.map &:url).to match_array(%w[a b])
       ^ Style/NestedParenthesizedCalls: Add parentheses to nested method call `links.map &:url`.

expect(list.select &:odd?).to eq [1, 5, 3]
       ^ Style/NestedParenthesizedCalls: Add parentheses to nested method call `list.select &:odd?`.

klass.for(attrs.keys.map &:intern)
          ^ Style/NestedParenthesizedCalls: Add parentheses to nested method call `attrs.keys.map &:intern`.

emails = Set.new(CourseUserDatum.joins(:user).where(course: @assessment.course).map &:email)
                 ^ Style/NestedParenthesizedCalls: Add parentheses to nested method call `CourseUserDatum.joins(:user).where(course: @assessment.course).map &:email`.

expect(accounts.map &:id).to eq TEST_ACCOUNTS.map &:id
       ^ Style/NestedParenthesizedCalls: Add parentheses to nested method call `accounts.map &:id`.

expect(vault.accounts.map &:id).to eq TEST_ACCOUNTS.map &:id
       ^ Style/NestedParenthesizedCalls: Add parentheses to nested method call `vault.accounts.map &:id`.

expect(@fixture.set_block &a).to eq(a)
       ^ Style/NestedParenthesizedCalls: Add parentheses to nested method call `@fixture.set_block &a`.
