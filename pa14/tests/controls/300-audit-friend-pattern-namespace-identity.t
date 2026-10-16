// expect exit 0
int ordinary() { return 7; }
int external();
template<class T> int identity(T value) { return value; }
template<class T> struct Owner {
  friend int ordinary();
  friend int external();
  friend int identity<>(int);
};
int external() { return 3; }
int main() {
  Owner<int> first;
  Owner<char> second;
  return ordinary() + external() + identity(4) - 14;
}
