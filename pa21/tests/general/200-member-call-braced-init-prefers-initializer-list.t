namespace std {
template<class T>
class initializer_list {
  const T * first;
  unsigned long count;
public:
  initializer_list() : first(0), count(0) {}
  unsigned long size() const { return count; }
  const T * begin() const { return first; }
  const T * end() const { return first + count; }
};
}

struct Item {
  Item(int) {}
  Item(const char *) {}
};

struct Value {
  Value(int) {}
  Value(const char *) {}
  Value(std::initializer_list<Item>) {}
};

struct Array {
  int insert(int, const Value&) { return 1; }
  int insert(int, Value&&) { return 2; }
  int insert(int, std::initializer_list<Item>) { return 3; }
};

int main()
{
  Array a;
  return a.insert(0, {1, "x"}) == 3 ? 0 : 1;
}
