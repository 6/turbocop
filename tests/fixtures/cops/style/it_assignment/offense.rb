it = 5
^^^^^^ Style/ItAssignment: Avoid assigning to local variable `it`, since `it` will be the default block parameter in Ruby 3.4+. Consider using a different variable name.
it = foo
^^^^^^^^ Style/ItAssignment: Avoid assigning to local variable `it`, since `it` will be the default block parameter in Ruby 3.4+. Consider using a different variable name.
it = bar(1, 2)
^^^^^^^^^^^^^^ Style/ItAssignment: Avoid assigning to local variable `it`, since `it` will be the default block parameter in Ruby 3.4+. Consider using a different variable name.

Thread.list.map(&:name).select { |it| it && it.include?('Profiling') }
                                  ^^ Style/ItAssignment: `it` is the default block parameter; consider another name.

expect(generate_result.find { |it| it[:name] == "rspec-core" }.fetch(:paths)).to contain_exactly(
                               ^^ Style/ItAssignment: `it` is the default block parameter; consider another name.
  "rspec-core",
)

samples_for_thread(all_samples, Thread.current)
  .map { |it| it.values.fetch(:"cpu-samples") }
          ^^ Style/ItAssignment: `it` is the default block parameter; consider another name.
  .reduce(:+)

def foo(it); end
        ^^ Style/ItAssignment: `it` is the default block parameter; consider another name.

def foo(it = 5); end
        ^^ Style/ItAssignment: `it` is the default block parameter; consider another name.

def foo(*it); end
         ^^ Style/ItAssignment: `it` is the default block parameter; consider another name.

def foo(it:); end
        ^^ Style/ItAssignment: `it` is the default block parameter; consider another name.

def foo(it: 5); end
        ^^ Style/ItAssignment: `it` is the default block parameter; consider another name.

def foo(**it); end
          ^^ Style/ItAssignment: `it` is the default block parameter; consider another name.

def foo(&it); end
         ^^ Style/ItAssignment: `it` is the default block parameter; consider another name.

it ,= @tag.upper_bound([tag, now])
^ Style/ItAssignment: `it` is the default block parameter; consider another name.

it ,= @tag.lower_bound([tag, key + 1])
^ Style/ItAssignment: `it` is the default block parameter; consider another name.

it ,= @tag.upper_bound([tag, key - 1])
^ Style/ItAssignment: `it` is the default block parameter; consider another name.

it ,= @tag.lower_bound([tag, INF])
^ Style/ItAssignment: `it` is the default block parameter; consider another name.

it ,= @tag.upper_bound([tag, 0])
^ Style/ItAssignment: `it` is the default block parameter; consider another name.

it ,= @tag.lower_bound([tag, okey])
^ Style/ItAssignment: `it` is the default block parameter; consider another name.

it ||= 0;  describe ||= 0
^ Style/ItAssignment: `it` is the default block parameter; consider another name.
