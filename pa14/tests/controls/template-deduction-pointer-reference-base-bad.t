// expect reject
template<class T> struct Base {};
struct Derived : Base<int> {};
template<class T> int select(Base<T>* const&) { return 0; }
int main() {
  Derived object;
  Derived* pointer = &object;
  return select(pointer);
}
