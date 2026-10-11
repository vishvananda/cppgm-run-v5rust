#include <locale>
#include <type_traits>
static_assert(std::is_base_of<std::locale::facet, std::num_put<char> >::value, "num_put : locale::facet");
static_assert(std::is_base_of<std::locale::facet, std::ctype<char> >::value, "ctype : locale::facet");
