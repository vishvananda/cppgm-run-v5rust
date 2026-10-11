// N3485 8.5.4p6, 12.2p5, 15.2p1: a std::initializer_list's array lives as
// long as the list, so an exception leaving the list's block from a
// condition's call destroys its elements before the handler runs.
namespace std {
template<class E>
class initializer_list {
  const E* first;
  unsigned long count;
  initializer_list(const E* p, unsigned long n) : first(p), count(n) {}
public:
  initializer_list() : first(0), count(0) {}
  unsigned long size() const { return count; }
};
}
struct Item {
  static int live;
  Item(int) { ++live; }
  Item(const Item&) { ++live; }
  ~Item() { --live; }
};
int Item::live = 0;
bool fail(unsigned long n) { if (n == 2) throw 1; return false; }
void hold() {
  std::initializer_list<Item> held = {4, 5};
  if (fail(held.size()))
    return;
}
int main() {
  try {
    hold();
  } catch (int) {
  }
  return Item::live;
}
