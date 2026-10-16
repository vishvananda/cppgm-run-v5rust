// N3485 9.2p2, 14.7.1, 14.8.2: expect exit 0
// A successful decltype trial against an unfinished class is transient. The
// same operand must see the newly declared member on its next occurrence.
template<class T> struct progress;
template<class T> typename progress<T>::type select(T);
char select(...);

template<class T> struct progress {
  typedef decltype(select(T())) before;
  typedef long type;
  typedef decltype(select(T())) after;
};

template<class T> typename progress<T>::type select(T value) { return value; }

int main() {
  long value = select(3);
  long (*first)(int) = select<int>;
  long (*second)(int) = select<int>;
  return sizeof(progress<int>::before) == 1 &&
         sizeof(progress<int>::after) == sizeof(long) &&
         first == second && value == 3 ? 0 : 1;
}
