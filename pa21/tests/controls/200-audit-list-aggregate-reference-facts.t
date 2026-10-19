// expect exit 0
namespace std { template<class T> class initializer_list; }
int live;
struct Item { Item() { ++live; } Item(const Item&) { ++live; } ~Item() { --live; } };
using Integers = std::initializer_list<int>;
using Objects = std::initializer_list<std::initializer_list<Item>>;
struct A { Integers values; };
struct B { Objects values; };
void local() {
  const A& a = {Integers{1, 2, 3}};
  const B& b = {Objects{{Item(), Item()}, {Item()}}};
  (void)a; (void)b;
}
void persistent() {
  static const A& a = {Integers{4, 5, 6}};
  static const B& b = {Objects{{Item(), Item()}, {Item()}}};
  (void)a; (void)b;
}
int main() { local(); persistent(); return live == 0 ? 0 : 1; }
