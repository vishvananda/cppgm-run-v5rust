struct tag {};
template<class T> struct iterator {
  iterator() {}
  iterator(tag, iterator const &) {}
  T * p;
};
iterator<int> make_iterator() { iterator<int> x; return x; }
int main() { make_iterator(); }
// VALIDATION: compile-pass
