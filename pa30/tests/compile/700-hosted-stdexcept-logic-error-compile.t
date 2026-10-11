#include <stdexcept>
#include <string>
#include <type_traits>
static_assert(std::is_base_of<std::exception, std::logic_error>::value, "logic_error : exception");
static_assert(std::is_constructible<std::logic_error, const std::string&>::value, "logic_error(const string&)");
