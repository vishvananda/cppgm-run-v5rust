#include <regex>
#include <cstddef>
#include <type_traits>
static_assert(std::is_same<std::cregex_iterator::difference_type, std::ptrdiff_t>::value, "cregex_iterator difference_type");
