// expect reject
template<class T> int call(T dependent) { return later(dependent); }
int later(int) { return 7; }
int main() { return call(1); }
