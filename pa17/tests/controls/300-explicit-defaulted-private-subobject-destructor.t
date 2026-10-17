// expect exit 0
// A nontrivial defaulted destructor root follows the synthesized emission path.
class hidden {
  ~hidden() {}
};
template<class T> struct box {
  T value;
  ~box() = default;
};
template box<hidden>::~box();
int main() { return 0; }
