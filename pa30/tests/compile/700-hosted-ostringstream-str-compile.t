#include <sstream>
#include <string>
#include <utility>
#include <type_traits>
static_assert(std::is_same<decltype(std::declval<std::ostringstream&>().str()), std::string>::value, "ostringstream::str() -> string");
