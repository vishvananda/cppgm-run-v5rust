namespace std {
template<class T>
class initializer_list {
  const T * first;
  unsigned long count;
public:
  unsigned long size() const { return count; }
  const T * begin() const { return first; }
  const T * end() const { return first + count; }
};
}

struct item {
  int key;
  int value;
  item(int k, int v) : key(k), value(v) {}
};

struct Box {
  int selected;
  Box(const Box&) : selected(2) {}
  Box(Box&&) : selected(3) {}
  Box(std::initializer_list<item>) : selected(1) {}
};

int main()
{
  Box box{{ {1, 2}, {3, 4} }};
  return box.selected == 1 ? 0 : 1;
}
