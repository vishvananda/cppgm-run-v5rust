template<class T, unsigned N> T (&cast(char const (&)[N]))[N];
template<class T> void use() { cast<T>(""); }
int main() { use<char>(); }
