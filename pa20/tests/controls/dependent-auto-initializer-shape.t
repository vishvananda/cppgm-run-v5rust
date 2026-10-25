// expect exit 0
int value = 7;
struct Source {
  int* operator()() const { return &value; }
};
template<class T> int read(T source) {
  if (auto* pointer = source()) {
    auto& reference = *pointer;
    auto first = source(), second = pointer;
    return reference + *first + *second;
  }
  return 0;
}
template<class T> int dormant(T source) {
  if (auto* pointer = source()) return *pointer;
  return 0;
}
int main() { return read(Source()) == 21 ? 0 : 1; }
