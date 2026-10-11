#include <iomanip>
#include <ostream>
#include <utility>
#include <type_traits>
static_assert(std::is_same<decltype(std::declval<std::ostream&>() << std::setprecision(2)), std::ostream&>::value, "ostream << setprecision -> ostream&");
