// N3485 14.6.2, 14.8.2: dependent value operands retain their qualified
// spelling for ABI encoding and resolve only on signature demand; expect exit 0
struct policy {
  static int value;
  struct nested { static short value; };
};
template<class T> auto direct(T) -> decltype(T::value) { return 3; }
template<class T> auto chained(T) -> decltype(T::nested::value) { return 5; }
template<class T> int bound(char (&)[sizeof(T::value)]) { return 7; }
template<class T> T&& declval();
template<class T> auto qualified(T) -> decltype(declval<T>().T::value, char()) {
  return 9;
}
template<class T> auto fixed(T) -> decltype(char()) { return 0; }
int main() {
  char bytes[sizeof(int)];
  return direct(policy()) == 3 && chained(policy()) == 5
      && bound<policy>(bytes) == 7 && qualified(policy()) == 9
      && fixed(policy()) == 0 ? 0 : 1;
}
