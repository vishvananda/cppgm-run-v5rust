// N3485 14.8.2.4p8 (standard example): g(Tuple<T1, Types...>) is more
// specialized than g(Tuple<Types...>), so g(Tuple<int, float>()) calls #2;
// expect exit 0
template<class...> struct Tuple {};
template<class... Types> int g(Tuple<Types...>) { return 1; }
template<class T1, class... Types> int g(Tuple<T1, Types...>) { return 2; }
int main() { return (g(Tuple<>()) == 1 && g(Tuple<int, float>()) == 2) ? 0 : 1; }
