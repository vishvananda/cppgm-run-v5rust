namespace N { enum R { value }; template<R> int f() { return 0; } }
int main() { using namespace N; return f<value>(); }
