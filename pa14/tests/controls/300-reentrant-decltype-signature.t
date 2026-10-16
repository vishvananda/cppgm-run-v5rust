// N3485 9.2p2, 14.7.1, 14.8.2: expect exit 0
// Re-entering a trailing-result operand after starting a class definition must
// use the members already declared there and retain one function identity.
template<class T> struct holder;
template<class T> auto pick(T) -> decltype(static_cast<typename holder<T>::type>(0));
char pick(...);

template<class T> struct holder {
  typedef long type;
  typedef decltype(pick(T())) result;
};

template<class T> auto pick(T value) -> decltype(static_cast<typename holder<T>::type>(0)) {
  return value;
}

int main() {
  long value = pick(4);
  long (*function)(int) = pick<int>;
  return sizeof(holder<int>::result) == sizeof(long) &&
         function == &pick<int> && value == 4 ? 0 : 1;
}
