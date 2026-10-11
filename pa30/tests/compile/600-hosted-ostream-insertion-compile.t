#include <ostream>
#include <utility>
#include <type_traits>
static_assert(std::is_same<decltype(std::declval<std::ostream&>() << 1), std::ostream&>::value, "ostream << int -> ostream&");
static_assert(std::is_same<decltype(std::declval<std::ostream&>() << "x"), std::ostream&>::value, "ostream << const char* -> ostream&");
