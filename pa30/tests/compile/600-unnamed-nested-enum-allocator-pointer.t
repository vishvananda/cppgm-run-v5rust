#include <memory>
#include <string>
#include <unordered_map>
#include <type_traits>
static_assert(std::is_same<std::allocator_traits<std::allocator<int> >::pointer, int*>::value, "allocator_traits pointer");
namespace {
struct Outer {
  enum E { A, B };
  std::allocator_traits<std::allocator<std::pair<unsigned long const, std::unordered_map<std::string, Outer::E> > > >::pointer p;
};
}
