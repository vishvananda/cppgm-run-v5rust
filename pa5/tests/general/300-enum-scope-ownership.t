enum E { value };
enum E;
enum E:int;
template<class T> enum word { member };
template<class T> enum class scoped_word { member };
template<class T> struct P {};
enum P<int> { other };
void f() { word *p; scoped_word *q; sizeof(P<int>); sizeof(E::value); }
