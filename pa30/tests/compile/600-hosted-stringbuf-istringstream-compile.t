#include <sstream>
#include <type_traits>
static_assert(std::is_base_of<std::streambuf, std::stringbuf>::value, "stringbuf : streambuf");
static_assert(std::is_base_of<std::istream, std::istringstream>::value, "istringstream : istream");
