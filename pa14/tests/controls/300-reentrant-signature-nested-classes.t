// N3485 9.2p2, 14.7.1, 14.8.2: expect exit 0
// Two class definitions remain active while a function's return substitution
// re-enters the first class and finds its already declared type.
template<class T> struct first;
template<class T> struct second;
template<class T> typename first<T>::type left(T);
template<class T> typename second<T>::type right(T);
char left(...);
char right(...);

template<class T> struct first {
  typedef long type;
  typedef decltype(right(T())) other;
};

template<class T> struct second {
  typedef long type;
  typedef decltype(left(T())) other;
};

template<class T> typename first<T>::type left(T value) { return value; }
template<class T> typename second<T>::type right(T value) { return value; }

int main() {
  long value = left(2);
  return sizeof(first<int>::other) == sizeof(long) &&
         sizeof(second<int>::other) == sizeof(long) &&
         value == 2 && right(3) == 3 ? 0 : 1;
}
