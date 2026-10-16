// expect exit 0
template<class T> struct Base {};
struct Middle : Base<int> {};
struct Derived : Middle {};
template<class T> int value(Base<T>) { return sizeof(T); }
template<class T> int pointer(const Base<T>*) { return sizeof(T); }
template<class T> int reference(const Base<T>&) { return sizeof(T); }
template<class T> int rvalue(Base<T>&&) { return sizeof(T); }
template<class T> struct Generic : Base<T> {};
int main() {
  Derived object;
  Generic<char> other;
  return value(object) + pointer(&object) + reference(object)
    + rvalue(static_cast<Derived&&>(object)) + pointer(&other) - 17;
}
