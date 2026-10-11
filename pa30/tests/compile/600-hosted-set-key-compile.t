#include <set>
#include <type_traits>
static_assert(std::is_same<std::set<int>::key_type, int>::value, "set key_type");
static_assert(std::is_same<std::set<int>::value_type, int>::value, "set value_type");
