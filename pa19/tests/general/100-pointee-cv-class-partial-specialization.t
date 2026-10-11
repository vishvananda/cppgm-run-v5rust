// Expected: class partial specialization matching must not treat pointee cv
// as ignorable top-level cv.

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

struct P {};

int main()
{
  return Match<const P*, const P*>::value == 1 ? 0 : Match<const P*, const P*>::value;
}
