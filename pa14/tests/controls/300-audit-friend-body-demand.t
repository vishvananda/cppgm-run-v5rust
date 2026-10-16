// expect exit 0
template<class T> struct Owner {
  friend int read(Owner) { return sizeof(T); }
  friend int unused(Owner) { return T::missing; }
};
int main() {
  Owner<int> first;
  Owner<char> second;
  return read(first) + read(second) - 5;
}
