# Converts KaTeX's screenshot test data to the JSON list of cases that
# examples/corpus.rs, the Python tools here and tests/katex_corpus.rs read.
#
# Usage: ruby tools/ss_data_to_json.rb ss_data.yaml out.json
#
# ss_data.yaml is KaTeX's test/screenshotter/ss_data.yaml. Each case becomes
# {name, tex, display, options}, where options holds its other keys (macros,
# pre, post, styles, noThrow and so on). Ruby, because its standard library
# reads YAML.
require 'json'
require 'yaml'

yaml_path, out_path = ARGV
abort 'Usage: ruby tools/ss_data_to_json.rb ss_data.yaml out.json' unless out_path

data = YAML.safe_load(File.read(yaml_path, encoding: 'UTF-8'))
cases = data.map do |name, item|
  item = { 'tex' => item } if item.is_a?(String)
  abort "#{name}: tex is not a string" unless item['tex'].is_a?(String)
  abort "#{name}: display is #{item['display'].inspect}" unless [nil, 1].include?(item['display'])
  %w[pre post styles].each do |key|
    abort "#{name}: #{key} is not a string" if item.key?(key) && !item[key].is_a?(String)
  end
  {
    'name' => name,
    'tex' => item['tex'],
    'display' => item['display'] == 1,
    'options' => item.reject { |key, _| %w[tex display].include?(key) },
  }
end
# Older json gems spread an empty object over two lines. A newline in the
# output is always between tokens, never inside a string.
json = JSON.pretty_generate(cases).gsub(/\{\n\s*\}/, '{}')
File.write(out_path, json + "\n")
puts "#{cases.size} cases in #{out_path}"
