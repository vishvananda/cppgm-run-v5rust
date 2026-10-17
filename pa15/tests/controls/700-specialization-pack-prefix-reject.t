// expect reject
// N3485 14.8.1p9: the deduced pack must start with the explicit prefix.
template<class... T> int choose(T...) { return 1; }
template<> int choose<long>(int, long) { return 7; }
int main() { return choose(1, 2L); }
