// N3485 14.7.3p1/p16: an explicitly specialized member destructor is the
// destructor of every variant (base-object D2 with a virtual base too);
// expect exit 0
static int which = 0;
template<class T> struct V { ~V() {} };
template<class T> struct B : virtual V<T> { ~B(); };
template<class T> B<T>::~B() { which = 1; }
template<> B<short>::~B() { which = 2; }
struct D : B<short> { };
int main() { { D d; } return which == 2 ? 0 : 1; }
