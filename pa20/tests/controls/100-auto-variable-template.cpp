// PA17 variable templates + N3485 7.1.6.4p3: an auto variable template deduces
// its type per specialization from the initializer; expect exit 0
template <class T> auto val = T(7);
template <class T> constexpr auto size_of = sizeof(T);
int main() { return (val<int> == 7 && size_of<int> == 4) ? 0 : 1; }
