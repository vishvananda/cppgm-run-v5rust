// N3485 14.8.2.4p12 (example): g(Tuple<int>()) calls g(Tuple<T1, Types&...>)
// (#3), more specialized than #1 and #2; expect exit 0
template<class...> struct Tuple {};
template<class... Types> int g(Tuple<Types...>) { return 1; }
template<class T1, class... Types> int g(Tuple<T1, Types...>) { return 2; }
template<class T1, class... Types> int g(Tuple<T1, Types&...>) { return 3; }
int main() { return g(Tuple<int>()) == 3 ? 0 : 1; }
