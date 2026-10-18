// N3485 14.8.2p8: the actual call still checks the invalid signature;
// expect reject
namespace ns { template<class T> typename T::missing make(); }
template<class T> auto active(T t) -> decltype(ns::make<int>(), t) { return t; }
int main() { return active(1); }
