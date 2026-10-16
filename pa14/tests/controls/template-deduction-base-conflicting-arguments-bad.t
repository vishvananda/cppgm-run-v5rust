// expect reject
template<class T> struct Base {};
struct Derived : Base<int> {};
template<class T> int select(Base<T>, T) { return 0; }
int main() {
  Derived object;
  return select(object, char(0));
}
