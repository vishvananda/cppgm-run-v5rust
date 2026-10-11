// N3485 14.7.3p16, 14.5.5.3p2: `template<> template<class U> struct
// O<int>::I<U*>` declares a partial specialization of the member template I of
// the implicit specialization O<int>; O<long> and O<int>'s other arguments
// keep the primary member template and the class template's member partial
// specializations; expect exit 0
template<class T> struct O {
  template<class U> struct I { static const int v = 0; };
  template<class U> struct I<U&> { static const int v = 7; };
};
template<> template<class U> struct O<int>::I<U*> { static const int v = 5; };
int main() { return O<long>::I<char*>::v + O<int>::I<char>::v + (O<int>::I<char&>::v == 7 ? 0 : 1); }
