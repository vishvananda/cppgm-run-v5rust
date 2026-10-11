// N3485 14.7.1/14.8.2: substituting g<int>'s return type instantiates A<int>,
// whose member uses g(T()) again; A<int>::type is already declared, so g<int>
// is viable there (not circular); expect exit 0
template<class T> struct A;
template<class T> typename A<T>::type g(T);
template<class T> struct A { typedef long type; static const int n = sizeof(g(T())); };
template<class T> typename A<T>::type g(T t) { return t + 1; }
int main() { return (g(1) == 2 && A<int>::n == sizeof(long)) ? 0 : 1; }
