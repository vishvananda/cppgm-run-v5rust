#include <vector>
#include <type_traits>
#include <cstddef>
using V = std::vector<int>;
static_assert(std::is_same<V::value_type, int>::value, "value_type");
static_assert(std::is_constructible<V, std::size_t>::value, "size-count constructor");
