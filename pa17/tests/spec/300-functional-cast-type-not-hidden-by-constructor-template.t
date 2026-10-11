// N3485 focus: 3.4.1 [basic.lookup.unqual], 5.2.3 [expr.type.conv]
template<class T> struct O;
template<class T> struct O<T&> { O(); O(T&); };
template<class T> struct O {
  T value;
  template<class U> O(U&&);
  operator O<T const&>() const& {
    return true ? O<T const&>(value) : O<T const&>();
  }
};
void use(O<int const&>);
int main() { O<int> value(0); use(value); }
