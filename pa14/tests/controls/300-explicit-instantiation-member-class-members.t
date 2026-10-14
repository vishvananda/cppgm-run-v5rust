// N3485 14.7.2p8: an explicit instantiation definition of Counter<long> also
// instantiates its member class Inner and Inner's static data member (its
// initializer runs); expect exit 0
int trace = 0;
int mark(int bit) { trace |= bit; return bit; }
template<class T> struct Counter { static int total; struct Inner { static int count; }; };
template<class T> int Counter<T>::total = mark(1);
template<class T> int Counter<T>::Inner::count = mark(4);
template struct Counter<long>;
int main() { return trace == 5 ? 0 : 1; }
