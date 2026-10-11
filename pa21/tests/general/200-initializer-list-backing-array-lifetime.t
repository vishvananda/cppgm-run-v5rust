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

int observe(std::initializer_list<Item>) {
  return Item::live;
}

int main() {
  if (observe({1, 2, 3}) != 3) return 1;
  if (Item::live != 0) return 2;

  {
    std::initializer_list<Item> held = {4, 5};
    if (held.size() != 2 || Item::live != 2) return 3;
  }

  return Item::live == 0 ? 0 : 4;
}
