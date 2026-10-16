// N3485 14.8.2p7-p8: explicit substitution evaluates a leading return
// before a later invalid parameter; its required default is a hard error;
// expect reject
template<class T> int required(T, int = T::missing);
template<class T> decltype(required(T())) ordered(T x, decltype(x.missing()) unused);
int main() { return sizeof(ordered<int>(1, 2)); }
