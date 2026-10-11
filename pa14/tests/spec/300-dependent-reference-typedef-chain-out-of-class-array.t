// VALIDATION: compile-pass
// N3485 focus: 14.5.1.3 [temp.static], 14.6.2.1 [temp.dep.type]

template<class T> struct next { typedef typename T::next type; };
template<class T> struct deref { typedef typename T::type type; };

template<class S> struct chars {
  typedef typename S::begin i0;
  typedef typename next<i0>::type i1;
  typedef typename next<i1>::type i2;
  static int data[3];
};

template<class S> int chars<S>::data[3] = {
  deref<i0>::type::value,
  deref<i1>::type::value,
  deref<i2>::type::value
};

int main() {}
