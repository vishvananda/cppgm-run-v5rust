template<class, class> struct pair {};
template<class> struct box {};
template<class T> using const_pair = pair<int, const T>;
template<class T> void accept(box<const_pair<T> >) {}
int main() { accept<int>(box<pair<int, const int> >()); }
