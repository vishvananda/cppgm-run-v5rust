int object;
int f(const int& = {object});
int g(const long& = {3});
int h(const int(&)[2] = {1, 2});
int i(int&& = {object});
int call() { return f() + g() + h() + i(); }
