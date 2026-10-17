// A private conversion rejects the constructor template's condition, preserving
// the conversion's semantic body demand while choosing the ellipsis overload.
// expect exit 0
struct Flag {
private:
  constexpr operator bool() const { return true; }
};
struct A {
  template<class T> explicit(T()) A(T) {}
  A(...) {}
};
int main() { Flag flag; A value(flag); return 0; }
