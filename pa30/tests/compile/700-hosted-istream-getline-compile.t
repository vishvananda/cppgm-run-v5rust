#include <istream>
#include <string>
#include <utility>
#include <type_traits>
static_assert(std::is_same<decltype(std::getline(std::declval<std::istream&>(), std::declval<std::string&>())), std::istream&>::value, "getline(istream&, string&) -> istream&");
