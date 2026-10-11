int count;
struct member {
  member() { ++count; }
  member(member const&) { ++count; }
  ~member() { --count; }
};
struct iterator {
  member value;
  iterator& operator++() { return *this; }
};
iterator at() { return iterator(); }
iterator end() { return iterator(); }
iterator begin(bool empty) { return empty ? end() : ++at(); }
int main() {
  (void)begin(false);
  return count;
}
