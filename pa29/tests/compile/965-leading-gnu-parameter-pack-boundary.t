template<class... T> void by_gnu(__attribute__((aligned(sizeof(T)))) int...);
int main() { return 0; }
