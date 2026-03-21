# frozen_string_literal: true

# Monkey-patch for the corpus oracle: rescue parser crashes in RuboCop's
# file processing loop so that one crashing file doesn't kill inspection
# of all subsequent files. Load via: --require ./bench/corpus/rescue_parser_crashes.rb
#
# Without this, a Prism::Translation::Parser crash (e.g., RegexpError on
# invalid multibyte escapes in jruby test files) propagates through
# process_file -> each_inspected_file and aborts the entire run, dropping
# every file after the crash from the JSON output.

module RuboCop
  class Runner
    private

    alias_method :original_process_file, :process_file

    def process_file(file)
      original_process_file(file)
    rescue InfiniteCorrectionLoop
      raise # let RuboCop's own handler deal with this
    rescue StandardError => e
      warn "#{Rainbow('[corpus] parser crash rescued').yellow} #{file}: #{e.class}: #{e.message}"
      []
    end
  end
end
