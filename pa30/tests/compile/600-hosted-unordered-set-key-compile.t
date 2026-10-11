#include <unordered_set>
#include <type_traits>
static_assert(std::is_same<std::unordered_set<int>::key_type, int>::value, "unordered_set key_type");
static_assert(std::is_same<std::unordered_set<int>::value_type, int>::value, "unordered_set value_type");
