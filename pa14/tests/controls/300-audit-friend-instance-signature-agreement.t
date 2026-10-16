// expect exit 0
template<class T> struct Owner { friend int read(Owner); };
int read(Owner<int>) { return 7; }
int read(Owner<char>) { return 3; }
int main() {
  Owner<int> first;
  Owner<char> second;
  return read(first) + read(second) - 10;
}
