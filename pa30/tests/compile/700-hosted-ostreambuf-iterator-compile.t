#include <iterator>
#include <ostream>
#include <type_traits>
static_assert(std::is_same<std::ostreambuf_iterator<char>::char_type, char>::value, "ostreambuf_iterator char_type");
