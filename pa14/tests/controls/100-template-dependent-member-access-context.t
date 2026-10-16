// N3485 11p4, 14.8.2p8: equivalent operands share structural identity, but
// deduction checks access in each template's context; expect exit 0
template<class T> auto granted(T) -> decltype(T::value);
template<class T> auto denied(T) -> decltype(T::value);
class policy {
  static int value;
  template<class T> friend auto granted(T) -> decltype(T::value);
};
template<class T> auto granted(T) -> decltype(T::value) { return 3; }
long denied(...);
class hidden { typedef int type; };
template<class T> typename T::type explicit_use(T) { return 5; }
template int explicit_use<hidden>(hidden);
int main() {
  return granted(policy()) == 3 && sizeof(denied(policy())) == sizeof(long)
      && explicit_use(hidden()) == 5 ? 0 : 1;
}
