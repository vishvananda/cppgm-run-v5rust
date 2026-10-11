template<class T> struct Box;
typedef Box<int> IntBox;

template<class T> struct Traits;
template<class T> struct Probe {
  typedef typename Traits<T>::type type;
  static type value() { return Traits<T>::value(); }
  static int dormant() { return T::missing; }
};
Probe<int>* deferred;
template<class T> struct Traits {
  typedef T type;
  static T value() { return 7; }
};

int main() { return Probe<int>::value() != 7; }
