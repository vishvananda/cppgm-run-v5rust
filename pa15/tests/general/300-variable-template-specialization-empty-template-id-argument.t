template <class T = void, class U = T>
struct less {};

template <class C>
inline const bool flag = false;

template <>
inline const bool flag<less<> > = true;

static_assert(flag<less<> >, "");

int main() { return 0; }
