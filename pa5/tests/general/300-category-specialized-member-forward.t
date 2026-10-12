template<class T> struct P;
template<> struct P<int> { struct member; };
struct P<int>::member { typedef int word; };

void syntax() { P<int>::member::word *pointer; }
