namespace std {
template<class T> class initializer_list {
  const T* first; unsigned long count;
  initializer_list(const T* p, unsigned long n) : first(p), count(n) {}
public:
  const T* begin() const { return first; }
};
}

int live;
struct item {
  int value;
  item(int x) : value(x) { ++live; }
  ~item() { --live; }
};

std::initializer_list<item> values{1, 2};

int main() {
  return live != 2 || values.begin()[0].value != 1 ||
         values.begin()[1].value != 2;
}
