// VALIDATION: compile-pass
// Collecting an out-of-class member-template definition may revisit a
// using-declaration while its base class is still dependent.

template<class>
struct base {
  int operator[](int);
};

template<class T>
struct derived : base<T> {
  typedef base<T> base_type;
  using base_type::operator[];
  template<class> void member();
};

template<class T>
template<class>
void derived<T>::member() {}
