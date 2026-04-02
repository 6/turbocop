my_method(1) \
^^^^^^^^^^^^^^ Layout/RedundantLineBreak: Redundant line break detected.
  [:a]

foo && \
^^^^^^^^ Layout/RedundantLineBreak: Redundant line break detected.
  bar

foo || \
^^^^^^^^ Layout/RedundantLineBreak: Redundant line break detected.
  bar

my_method(1,
^^^^^^^^^^^^ Layout/RedundantLineBreak: Redundant line break detected.
          2,
          "x")

foo(' .x')
^^^^^^^^^^ Layout/RedundantLineBreak: Redundant line break detected.
  .bar
  .baz

a =
^^^ Layout/RedundantLineBreak: Redundant line break detected.
  m(1 +
    2 +
    3)

b = m(4 +
^^^^^^^^^ Layout/RedundantLineBreak: Redundant line break detected.
      5 +
      6)

raise ArgumentError,
^^^^^^^^^^^^^^^^^^^^ Layout/RedundantLineBreak: Redundant line break detected.
      "can't inherit configuration from the rubocop gem"

foo(x,
^^^^^^ Layout/RedundantLineBreak: Redundant line break detected.
    y,
    z)
  .bar
  .baz

x = [
^^^^^ Layout/RedundantLineBreak: Redundant line break detected.
  1,
  2,
  3
]

y = {
^^^^^ Layout/RedundantLineBreak: Redundant line break detected.
  a: 1,
  b: 2
}

foo(
^^^^ Layout/RedundantLineBreak: Redundant line break detected.
  bar(1, 2)
)

@count +=
^^^^^^^^^ Layout/RedundantLineBreak: Redundant line break detected.
  items.size

@@total +=
^^^^^^^^^^ Layout/RedundantLineBreak: Redundant line break detected.
  items.size

$counter +=
^^^^^^^^^^^ Layout/RedundantLineBreak: Redundant line break detected.
  items.size

@cache ||=
^^^^^^^^^^ Layout/RedundantLineBreak: Redundant line break detected.
  compute_value

@flag &&=
^^^^^^^^^ Layout/RedundantLineBreak: Redundant line break detected.
  check_flag

# Multiline regex — RuboCop's safe_to_split? does not check :regexp,
# so assignments containing multiline regexps are still flaggable.
pattern = /
^^^^^^^^^^ Layout/RedundantLineBreak: Redundant line break detected.
  \A
  (?<key>.+)
  \z
/x

# Multiline %w array — RuboCop's safe_to_split? does not check arrays.
names = %w[
^^^^^^^^^^^ Layout/RedundantLineBreak: Redundant line break detected.
  alpha
  beta
  gamma
]

loop do
  if scan_progress_busy_duration > queue_timeout.to_i
    scan_progress_resp[:products].select { |p| p[:status] == 'B' }.each do |p|
      PWN::Plugins::BlackDuckBinaryAnalysis.abort_product_scan(
      ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Layout/RedundantLineBreak: Redundant line break detected.
        token: token,
        product_id: p[:product_id]
      )
    end
  end
end

public_class_method def self.get_uris(opts = {})
  search_results = opts[:search_results]

  search_results.map do |search_results_hash|
    extract_uris(
    ^^^^^^^^^^^^^ Layout/RedundantLineBreak: Redundant line break detected.
      search_results_hash: search_results_hash
    )
  end.flatten
rescue StandardError => e
  raise e
end

scan_resp[:signals].each do |signal|
  cmd(
  ^^^^ Layout/RedundantLineBreak: Redundant line break detected.
    gqrx_sock: gqrx_sock,
    cmd: "M #{mode_str} #{passband_hz}",
    resp_ok: 'RPRT 0'
  )
end

if dev_dependency_arr.include?(gem_name.to_sym)
  spec.add_development_dependency(
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Layout/RedundantLineBreak: Redundant line break detected.
    gem_name,
    gem_version
  )
else
  spec.add_dependency(
  ^^^^^^^^^^^^^^^^^^^ Layout/RedundantLineBreak: Redundant line break detected.
    gem_name,
    gem_version
  )
end
