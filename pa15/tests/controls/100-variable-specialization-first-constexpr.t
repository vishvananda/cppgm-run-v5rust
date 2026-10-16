// A canonical shell keeps its first constexpr fact; expect exit 0
template<class T> constexpr int* value = nullptr;
using Seen = decltype(value<int>);
template<> int* const value<int> = nullptr;
static_assert(value<int> == nullptr, "shell retains constexpr");

struct Owner {
  template<class T> static int* const member;
};
template<> constexpr int* Owner::member<int>;
template<> int* const Owner::member<int> = nullptr;
static_assert(Owner::member<int> == nullptr, "declaration retains constexpr");
int main() { return 0; }
