template<class T> struct storage { T value; };
template<class T> struct result : storage<T> {};
struct item { friend result<item> make(); };
int main() { return 0; }
