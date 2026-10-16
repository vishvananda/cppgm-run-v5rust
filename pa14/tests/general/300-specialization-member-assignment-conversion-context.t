// The declaring class determines assignment's literal widening convention.
struct plain { long inherited; };
template<class T> struct cell : plain {
  T own;
  static long shared;
  struct nested { long value; };
  void assign() { own = 2; inherited = 3; shared = 4; }
};
template<class T> long cell<T>::shared;
struct derived : cell<long> { long extra; };

int main() {
  derived object;
  cell<long>::nested nested;
  object.own = 5;
  object.inherited = 6;
  object.extra = 7;
  object.shared = 8;
  nested.value = 9;
  object.own = {10};
  object.assign();
  return object.own == 2 && object.inherited == 3 && object.shared == 4
      && object.extra == 7 && nested.value == 9 ? 0 : 1;
}
