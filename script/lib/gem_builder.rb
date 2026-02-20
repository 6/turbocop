# frozen_string_literal: true

require "fileutils"
require "tmpdir"

# Builds turbocop gems — both base (fallback) and platform-specific variants.
#
# Base gem: no binary, users on unsupported platforms get a helpful error.
# Platform gem: includes precompiled binary in libexec/.
class GemBuilder
  GEM_SOURCE = File.expand_path("../../gems/turbocop", __dir__)

  attr_reader :version, :platform, :binary_path

  # @param version [String] gem version (e.g. "0.1.0")
  # @param platform [String, nil] platform string (e.g. "arm64-darwin"), nil for base gem
  # @param binary_path [String, nil] path to compiled binary, required for platform gems
  def initialize(version:, platform: nil, binary_path: nil)
    @version = version
    @platform = platform
    @binary_path = binary_path
  end

  def platform?
    !platform.nil?
  end

  # Assembles gem contents into the given directory. Useful for testing.
  # Returns the directory path.
  def assemble(dir)
    FileUtils.cp_r(File.join(GEM_SOURCE, "lib"), dir)
    FileUtils.cp_r(File.join(GEM_SOURCE, "exe"), dir)

    if platform?
      raise "Binary not found: #{binary_path}" unless File.file?(binary_path)

      libexec = File.join(dir, "libexec")
      FileUtils.mkdir_p(libexec)
      FileUtils.cp(binary_path, File.join(libexec, "turbocop"))
      FileUtils.chmod(0o755, File.join(libexec, "turbocop"))
    end

    patch_version(dir)
    write_gemspec(dir)
    dir
  end

  # Builds the .gem file and moves it to output_dir.
  # Returns the path to the built .gem file.
  def build(output_dir: Dir.pwd)
    Dir.mktmpdir("turbocop-gem-") do |tmpdir|
      assemble(tmpdir)

      Dir.chdir(tmpdir) do
        system("gem", "build", "turbocop.gemspec", exception: true)
      end

      gem_file = Dir.glob(File.join(tmpdir, "turbocop-*.gem")).first
      dest = File.join(output_dir, File.basename(gem_file))
      FileUtils.mv(gem_file, dest)
      dest
    end
  end

  # Returns the gemspec content as a string.
  def gemspec_content
    lines = []
    lines << '# frozen_string_literal: true'
    lines << ''
    lines << 'require_relative "lib/turbocop"'
    lines << ''
    lines << 'Gem::Specification.new do |spec|'
    lines << '  spec.name     = "turbocop"'
    lines << '  spec.version  = Turbocop::VERSION'
    lines << "  spec.platform = \"#{platform}\"" if platform?
    lines << '  spec.authors  = ["6"]'
    lines << ''
    lines << '  spec.summary     = "Fast Ruby linter targeting RuboCop compatibility"'
    if platform?
      lines << '  spec.description = "A Ruby linter written in Rust that reads your existing .rubocop.yml " \\'
      lines << '                     "and runs 900+ cops. " \\'
      lines << "                     \"Platform variant: #{platform}.\""
    else
      lines << '  spec.description = "A Ruby linter written in Rust that reads your existing .rubocop.yml " \\'
      lines << '                     "and runs 900+ cops."'
    end
    lines << '  spec.homepage    = "https://github.com/6/turbocop"'
    lines << '  spec.license     = "MIT"'
    lines << ''
    lines << '  spec.required_ruby_version = ">= 3.1.0"'
    lines << ''
    lines << '  spec.metadata["source_code_uri"] = spec.homepage'
    lines << '  spec.metadata["changelog_uri"]   = "#{spec.homepage}/releases"'
    lines << ''
    lines << '  spec.files       = Dir["lib/**/*", "exe/**/*", "libexec/**/*"]'
    lines << '  spec.bindir      = "exe"'
    lines << '  spec.executables = ["turbocop"]'
    lines << 'end'
    lines.join("\n") + "\n"
  end

  private

  def patch_version(dir)
    version_file = File.join(dir, "lib", "turbocop.rb")
    content = File.read(version_file)
    content.sub!(/VERSION = ".*"/, %(VERSION = "#{version}"))
    File.write(version_file, content)
  end

  def write_gemspec(dir)
    File.write(File.join(dir, "turbocop.gemspec"), gemspec_content)
  end
end
