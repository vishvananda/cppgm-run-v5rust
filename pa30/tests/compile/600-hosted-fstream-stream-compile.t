#include <fstream>
#include <ostream>
#include <istream>
#include <type_traits>
static_assert(std::is_base_of<std::ostream, std::ofstream>::value, "ofstream : ostream");
static_assert(std::is_base_of<std::istream, std::ifstream>::value, "ifstream : istream");
