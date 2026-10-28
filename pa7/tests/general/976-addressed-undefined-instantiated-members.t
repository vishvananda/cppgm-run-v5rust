template<class T> struct C {
  static void addressed();
  static void called();
  static void inside() {}
  static void outside();
  template<class U> static void specialized();
};
template<class T> void C<T>::outside() {}
void (*a)() = &C<int>::addressed;
void (*a_again)() = &C<int>::addressed;
void (*b)() = &C<int>::called;
void (*c)() = &C<int>::inside;
void (*d)() = &C<int>::outside;
void (*e)() = &C<int>::specialized<char>;
void use() { C<int>::called(); }
