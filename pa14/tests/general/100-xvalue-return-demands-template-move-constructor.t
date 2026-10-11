template<class T> struct box {
  int tag;
  box() : tag(0) {}
  box(box&&) : tag(1) {}
};
box<int> move_box(box<int>& x) { return static_cast<box<int>&&>(x); }
int main() {
  box<int> x;
  return move_box(x).tag != 1;
}
