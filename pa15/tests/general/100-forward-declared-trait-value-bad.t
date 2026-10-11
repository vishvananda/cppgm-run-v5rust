namespace std { template<class T> struct is_nothrow_default_constructible; }
struct Value {};
static_assert(std::is_nothrow_default_constructible<Value>::value, "undefined primary has no value");
int main() { return 0; }
