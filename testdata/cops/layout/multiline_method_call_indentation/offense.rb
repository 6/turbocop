foo
  .bar
    .baz
    ^^^ Layout/MultilineMethodCallIndentation: Align `.baz` with `foo` on line 1.

thing
  .first
  .second
      .third
      ^^^ Layout/MultilineMethodCallIndentation: Align `.third` with `thing` on line 5.

query
  .select('foo')
  .where(x: 1)
    .order(:name)
    ^^^ Layout/MultilineMethodCallIndentation: Align `.order` with `query` on line 10.

# Block chain continuation: .sort_by should align with .with_index dot
frequencies.map.with_index { |f, i| [f / total, hex[i]] }
           .sort_by { |r| -r[0] }
           ^^^ Layout/MultilineMethodCallIndentation: Align `.sort_by` with `.with_index` on line 16.

# Multiline receiver chain with single-line block: .sort_by should align with .with_index dot
submission.template_submitters
          .group_by.with_index { |s, index| s['order'] || index }
          .sort_by(&:first).pluck(1)
          ^^^ Layout/MultilineMethodCallIndentation: Align `.sort_by` with `.with_index` on line 21.
