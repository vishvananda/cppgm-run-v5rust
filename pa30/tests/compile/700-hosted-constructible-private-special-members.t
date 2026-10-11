#include <type_traits>
class blocked { blocked() = default; blocked(const blocked&) = default;
  blocked& operator=(const blocked&) = default; };
static_assert(!std::is_default_constructible<blocked>::value, "");
static_assert(!std::is_copy_constructible<blocked>::value, "");
static_assert(!std::is_move_constructible<blocked>::value, "");
static_assert(!std::is_copy_assignable<blocked>::value, "");
static_assert(!std::is_move_assignable<blocked>::value, "");
int main() {}
