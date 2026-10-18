// N3485 14.8.2p8: failed signature key resolution does not demand an unused
// template's body or diagnose its eventual call; expect exit 0
namespace ns { template<class T> typename T::missing make(); }
template<class T> auto dormant(T t) -> decltype(ns::make<int>(), t) { return t; }
int main() { return 0; }
