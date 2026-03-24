[1, 2, 3].count
          ^^^^^ Performance/Size: Use `size` instead of `count`.
{a: 1}.count
       ^^^^^ Performance/Size: Use `size` instead of `count`.
[].count
   ^^^^^ Performance/Size: Use `size` instead of `count`.
(1..3).to_a.count
            ^^^^^ Performance/Size: Use `size` instead of `count`.
[[:foo, :bar], [1, 2]].to_h.count
                            ^^^^^ Performance/Size: Use `size` instead of `count`.
Array[*1..5].count
             ^^^^^ Performance/Size: Use `size` instead of `count`.
Array(1..5).count
            ^^^^^ Performance/Size: Use `size` instead of `count`.
Hash[*('a'..'z')].count
                  ^^^^^ Performance/Size: Use `size` instead of `count`.
Hash(key: :value).count
                  ^^^^^ Performance/Size: Use `size` instead of `count`.
categories.to_a.count
                ^^^^^ Performance/Size: Use `size` instead of `count`.
post.comments.to_a.count
                   ^^^^^ Performance/Size: Use `size` instead of `count`.
[1, 2, 3]&.count
           ^^^^^ Performance/Size: Use `size` instead of `count`.
(1..3)&.to_a&.count
              ^^^^^ Performance/Size: Use `size` instead of `count`.
[[:foo, :bar], [1, 2]]&.to_h&.count
                              ^^^^^ Performance/Size: Use `size` instead of `count`.
# Multi-statement block — .count is NOT the direct body, still flagged
items.each { puts "hi"; [1, 2].count }
                               ^^^^^ Performance/Size: Use `size` instead of `count`.
# .to_a.count nested inside hash inside array inside single-statement block
data.map { |r| [r[:id], { 'count' => r['items'].to_a.count }] }
                                                     ^^^^^ Performance/Size: Use `size` instead of `count`.
# Chained .count inside single-statement block — .count is NOT the sole body
it "counts" do [:a, :b, :c].count.should == 3 end
                            ^^^^^ Performance/Size: Use `size` instead of `count`.
