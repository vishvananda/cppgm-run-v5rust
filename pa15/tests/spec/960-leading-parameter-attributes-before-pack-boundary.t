template<class... T> void by_type(alignas(T) int...);
template<class... T> void by_size(alignas(sizeof(T)) int...);
int main() { return 0; }
