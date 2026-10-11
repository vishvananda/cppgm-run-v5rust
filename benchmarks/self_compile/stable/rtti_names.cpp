#include "rtti_names.h"

using namespace std;
using namespace cpp_decl;

string rtti_symbol_for_display_name(const string & name)
{
  string out = "@__rtti_";
  for(size_t i = 0; i < name.size(); ++i) {
    const char ch = name[i];
    if((ch >= 'a' && ch <= 'z') ||
       (ch >= 'A' && ch <= 'Z') ||
       (ch >= '0' && ch <= '9') ||
       ch == '_') {
      out += ch;
    } else if(ch == ':' && i + 1 < name.size() && name[i + 1] == ':') {
      out += "__";
      ++i;
    } else {
      out += '_';
    }
  }
  return out;
}

string rtti_symbol_for_type(const TypePtr & type)
{
  return rtti_symbol_for_display_name(describe_type(type));
}
