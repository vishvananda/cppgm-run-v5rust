#include <sstream>
#include <utility>
#include <type_traits>
static_assert(std::is_same<decltype(std::declval<std::ostringstream&>().tellp()), std::ostream::pos_type>::value, "ostringstream::tellp() -> pos_type");
