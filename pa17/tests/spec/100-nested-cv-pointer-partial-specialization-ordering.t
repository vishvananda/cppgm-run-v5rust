// VALIDATION: compile-pass
// N3485 focus: 14.5.5.2 [temp.class.order]
// A nested const-pointer pattern is more specialized than the unqualified
// pointer pattern even though both can match by deducing different T types.

template<class T>
struct box {};

template<class T>
struct choose;

template<class T>
struct choose<box<T *> > {
  static const int value = 1;
};

template<class T>
struct choose<box<const T *> > {
  static const int value = 2;
};

int main()
{
  return choose<box<const char *> >::value == 2 ? 0 : 1;
}
