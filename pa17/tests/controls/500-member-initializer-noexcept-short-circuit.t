// expect exit 0
template<class T> struct Later { Later() noexcept(T::missing); };
int throwing();
struct Owner { int first=throwing(); Later<int> second; };
static_assert(!noexcept(Owner()), "earlier initializer suppresses later specification");
int main() { return 0; }
