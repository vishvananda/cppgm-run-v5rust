// expect exit 0
// Named defaulted constructor/copy/destructor roots need their definition and
// exception facts even if ordinary code never uses them.
template<class T>
struct box {
  T value;
  box() = default;
  box(const box&) = default;
  ~box() = default;
};

template box<int>::box();
template box<int>::box(const box<int>&);
template box<int>::~box();

int main() { return 0; }
