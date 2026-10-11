#include <string>
#include <tuple>
#include <type_traits>
using T = std::tuple<std::string&&>;
static_assert(std::tuple_size<T>::value == 1, "tuple<T&&> has one element");
static_assert(std::is_same<std::tuple_element<0, T>::type, std::string&&>::value,
              "element 0 is std::string&&");
