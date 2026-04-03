x = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
                                                                                                                        ^^^^^ Layout/LineLength: Line is too long. [125/120]
y = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
                                                                                                                        ^^^^^^^^^^ Layout/LineLength: Line is too long. [130/120]
z = "ccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc"
                                                                                                                        ^ Layout/LineLength: Line is too long. [121/120]

																				aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa
                                                                                                    ^ Layout/LineLength: Line is too long. [140/120]

# A commented string concatenation like <<'taint_tracer.js...' must not open a fake heredoc.
# RuboCop still checks the later long lines.
# expect(subject.digest).to eq('pt.browser.arachni/' <<'taint_tracer.js><SCRIPT src' <<
x = [
                                                       "function( name, value ){\n            document.cookie = name + \"=post-\" + value\n        }",
                                                                                                                        ^ Layout/LineLength: Line is too long. [150/120]
]

# expect(subject.elements_with_events).to eq('pt.browser.arachni/' <<'taint_tracer.js><SCRIPT src' <<
click_handlers = {
  "click" => [
                                "function( e ) {\n\t\t\t\t// Discard the second event of a jQuery.event.trigger() and\n\t\t\t\t// when an event is called after a page has unloaded\n\t\t\t\treturn typeof jQuery !== core_strundefined && (!e || jQuery.event.triggered !== e.type) ?\n\t\t\t\t\tjQuery.event.dispatch.apply( eventHandle.elem, arguments ) :\n\t\t\t\t\tundefined;\n\t\t\t}"
                                                                                                                        ^ Layout/LineLength: Line is too long. [382/120]
  ]
}

         "It's based on the following blog post: [https://medium.com/gett-engineering/rxswift-to-apples-combine-cheat-sheet-e9ce32b14c5b](https://medium.com/gett-engineering/rxswift-to-apples-combine-cheat-sheet-e9ce32b14c5b)\n\n"
                                                                                                                        ^ Layout/LineLength: Line is too long. [230/120]

      lines << %{<svg width="#{width}" height="#{height}" version="1.1" xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink">}
                                                                                                                        ^ Layout/LineLength: Line is too long. [151/120]

        5.times { |index| results << Hashie::Mash::Rash.new(title: "title #{index}", description: "content #{index}", path: "http://foo.gov/#{index}", changed: '2020-09-09 00:00:00 UTC', created: '2020-09-09 00:00:00 UTC', thumbnail_url: 'https://search.gov/img.svg') }
                                                                                                                        ^ Layout/LineLength: Line is too long. [269/120]

        5.times { |index| results << Hashie::Mash::Rash.new(title: "title #{index}", description: "content #{index}", path: "http://foo.gov/#{index}") }
                                                                                                                        ^ Layout/LineLength: Line is too long. [152/120]

      5.times { |index| results << Hashie::Mash::Rash.new(title: "title #{index}", description: "content #{index}", url: "http://foo.gov/#{index}") }
                                                                                                                        ^ Layout/LineLength: Line is too long. [149/120]

        5.times { |index| results << Hashie::Mash::Rash.new(title: "title #{index}", description: "content #{index}", url: "http://foo.gov/#{index}", published_at: twelve_years_ago, youtube_thumbnail_url: "http://youtube.com/#{index}", duration: '1:23') }
                                                                                                                        ^ Layout/LineLength: Line is too long. [255/120]

final_answer = "Here are some fascinating facts from the James Webb Space Telescope's Wikipedia page" # rubocop:disable Layout/LineLength
expect(agent.conversation_history).to eq([
          { "role" => "user", "content" => "Tell me some interesting facts from the James Webb Space Telescope's Wikipedia page" },
                                                                                                                        ^^^^^^^^^^^ Layout/LineLength: Line is too long. [131/120]
])

        raise Puppet::Network::HTTP::Error::HTTPNotFoundError.new("No route for #{req.method} #{req.path}", Puppet::Network::HTTP::Issues::HANDLER_NOT_FOUND)
                                                                                                                        ^ Layout/LineLength: Line is too long. [157/120]

raise Puppet::Network::HTTP::Error::HTTPNotFoundError.new(
                                                                  "  Supported /puppet API versions: #{Puppet::Network::HTTP::SERVER_URL_VERSIONS}\n",
                                                                                                                        ^ Layout/LineLength: Line is too long. [150/120]
                                                                  Puppet::Network::HTTP::Issues::HANDLER_NOT_FOUND
)

      raise Puppet::Network::HTTP::Error::HTTPBadRequestError.new(_("Missing required Accept header"), Puppet::Network::HTTP::Issues::MISSING_HEADER_FIELD)
                                                                                                                        ^ Layout/LineLength: Line is too long. [155/120]

    decoded  = '200904281000001|DOM|IND|INR|58.00|22|NULL|http://localhost:3000/orders/1/ed5230696ad525b9e322a6a64b56322e/done?utm_nooverride=1|http://hardcoregamer.localhost:3000|TOML'
                                                                                                                        ^ Layout/LineLength: Line is too long. [185/120]

    expected = '200904281000001|DOM|IND|INR|58.00|22|1|http://localhost:3000/orders/1/ed5230696ad525b9e322a6a64b56322e/done?utm_nooverride=1|http://hardcoregamer.localhost:3000|TOML'
                                                                                                                        ^ Layout/LineLength: Line is too long. [182/120]

			@checks ||= self.available_checks.collect { |c| perform_check c }.flatten(1) + children.collect(&:checks).flatten(1)
                                                                                                                     ^ Layout/LineLength: Line is too long. [122/120]
