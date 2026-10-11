// N3485 8.5.4p6, 15.2p1: a list's backing array lives as long as the list,
// so an exception from any later condition in its scope destroys the
// elements, last first, and the earlier automatic objects before the
// handler runs; a reference-bound list behaves the same way.
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
int live, order;
struct Item {
  int value;
  Item(int v) : value(v) { ++live; }
  ~Item() { --live; order = order * 10 + value; }
};
struct Guard {
  Guard() { ++live; }
  ~Guard() { --live; order = order * 10 + 9; }
};
bool fail(unsigned long n) { if (n == 2) throw 1; return false; }
unsigned long pick(unsigned long n) { if (n == 2) throw 2; return n; }
void in_if() { Guard g; std::initializer_list<Item> held = {4, 5}; if (fail(held.size())) return; }
void in_while() { Guard g; std::initializer_list<Item> held = {4, 5}; while (fail(held.size())) {} }
void in_for() { Guard g; std::initializer_list<Item> held = {4, 5}; for (; fail(held.size());) {} }
void in_switch() { Guard g; std::initializer_list<Item> held = {4, 5}; switch (pick(held.size())) { default: break; } }
void bound() { Guard g; const std::initializer_list<Item>& held = {4, 5}; if (fail(held.size())) return; }
int run(void (*f)())
{
  live = order = 0;
  try { f(); } catch (int) { return live == 0 && order == 549 ? 0 : 1; }
  return 1;
}
int main()
{
  return run(in_if) + 2 * run(in_while) + 4 * run(in_for) +
    8 * run(in_switch) + 16 * run(bound);
}
