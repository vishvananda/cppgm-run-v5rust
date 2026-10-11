template<class T> int foo();
int use(int (*)());
int main() { return use(foo<int>); }
