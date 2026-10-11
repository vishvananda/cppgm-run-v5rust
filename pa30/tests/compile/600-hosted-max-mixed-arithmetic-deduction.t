#include <algorithm>
#include <type_traits>
#include <utility>
static_assert(
  std::is_same<decltype(std::max(std::declval<unsigned long>(),
                                 std::declval<unsigned long>())),
               const unsigned long&>::value,
  "std::max(T,T) deduces T and returns const T&");
