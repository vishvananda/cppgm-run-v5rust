#include <sstream>
#include <string>

using namespace std;

#include "cppast_ast.h"
#include "cppast_dump.h"

namespace {

void dump_node(ostringstream & out, const CppAstNode & node, size_t depth)
{
  for(size_t i = 0; i < depth; ++i) {
    out << "  ";
  }

  out << cppast_kind_text(node.kind);
  const string value = cppast_display_value(node);
  if(!value.empty()) {
    out << ' ' << value;
  }
  out << '\n';

  for(const auto & child : node.children) {
    dump_node(out, child, depth + 1);
  }
}

}  // namespace

string describe_cppast_translation_unit(const CppAstNode & node)
{
  ostringstream out;
  dump_node(out, node, 0);
  return out.str();
}
