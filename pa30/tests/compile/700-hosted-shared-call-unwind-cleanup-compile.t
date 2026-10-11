#include <string>
#include <type_traits>
static_assert(std::is_same<std::string::value_type, char>::value, "<string> value_type");
struct E {};
extern void sink(const char *, const std::string &);
void probe() {
  try { throw E(); } catch(E &) { std::string s("a"); sink("x", s); }
  try { throw E(); } catch(E &) { std::string s("a"); sink("x", s); }
}
