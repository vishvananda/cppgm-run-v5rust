// Expected: recursive named-type deduction inside class partial
// specialization matching must preserve pointee cv on template-id bases.

template<class X, class Y>
struct Box {};

template<class A, class B>
struct Match {
  static const int value = 0;
};

template<class T>
struct Match<T*, T*> {
  static const int value = 1;
};

template<class T>
struct Match<const T*, T*> {
  static const int value = 2;
};

int main()
{
  return Match<const Box<int, long>*, const Box<int, long>*>::value == 1
             ? 0
             : Match<const Box<int, long>*, const Box<int, long>*>::value;
}
