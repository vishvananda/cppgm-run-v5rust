// expect exit 0
// Named defaulted assignment roots have synthesized definitions even if unused.
template<class T> struct box {
  T value;
  box& operator=(const box&) = default;
};
template box<int>& box<int>::operator=(const box<int>&);
int main() { return 0; }
