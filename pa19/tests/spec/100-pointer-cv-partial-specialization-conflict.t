// VALIDATION: compile-pass
// N3485 focus: 14.5.5.2 [temp.class.order]

template<class A, class B>
struct Match {
  static const int value = 0;
};

template<class T>
struct Match<T*, T*> {
  static const int value = 1;
};

template<class T>
struct Match<T*, const T*> {
  static const int value = 2;
};

struct P {};

int main()
{
  return Match<const P*, const P*>::value;
}
