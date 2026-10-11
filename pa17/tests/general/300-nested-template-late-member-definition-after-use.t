template<class T> struct outer { template<class U> struct inner; };
template<class T> template<class U>
struct outer<T>::inner { void call(); };
static_assert(sizeof(outer<int>::inner<long>), "");
template<class T> template<class U>
void outer<T>::inner<U>::call() {}
int main() { outer<int>::inner<long> value; value.call(); }
