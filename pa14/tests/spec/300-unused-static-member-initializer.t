// N3485 focus: 14.7.1 [temp.inst]/1,2,8,10: class use does not demand static definitions.
int calls;
int initialize() { ++calls; return 7; }
template<class T> struct unused {
  static int effect;
  static int invalid;
  int value() { return 7; }
  operator int() { return 3; }
};
template<class T> int unused<T>::effect = initialize();
template<class T> int unused<T>::invalid = T::missing;
int main() {
  unused<int> object;
  int converted = object;
  return calls || object.value() != 7 || converted != 3;
}
