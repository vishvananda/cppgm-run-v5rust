// N3485 8.3p1, 14.5.2p1, 14.5.6.1p5: an out-of-class member template
// definition must match a declared member (get(U*) vs get(U)); expect reject
template <class T> struct box { template <class U> int get(U); };
template <class T> template <class U> int box<T>::get(U*) { return 7; }
int main() { return 0; }
