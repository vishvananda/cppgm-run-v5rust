#include <unordered_map>

struct base {
  std::unordered_map<int, int> data;
  template<class = void> void register_types();
};
struct collection : base {};
template<class T> T&& declval();
template<class T> auto probe(int) -> decltype(
    declval<T&>().template register_types<>(), char{});
template<class> int probe(...);
static_assert(sizeof(probe<collection>(0)) == 1, "");
int main() {}
