// N3485 focus: 14.7.1 [temp.inst]/1,2,8,10: retain nested member definitions separately.
template<class T> struct outer {
  struct first { static int value; };
  struct second { static int value; };
  template<class U> struct inner { static int value, unused; };
};
template<class T> int outer<T>::first::value = 3;
template<class T> int outer<T>::second::value = T::missing;
template<class T> template<class U> int outer<T>::inner<U>::value = 7;
template<class T> template<class U> int outer<T>::inner<U>::unused = U::missing;
int read() { return outer<char>::inner<int>::value; }
int main() {
  outer<int>::second object;
  return outer<int>::first::value != 3 || read() != 7 || sizeof(object) == 0;
}
