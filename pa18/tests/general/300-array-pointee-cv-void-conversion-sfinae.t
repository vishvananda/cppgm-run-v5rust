template<class Y, class T> struct convertible {
  static char test(T *);
  static long test(...);
  enum { value = sizeof(test(static_cast<Y *>(0))) == 1 };
};
struct X {};
int main() { return convertible<const X[], void>::value; }
