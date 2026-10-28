template<class T> int f(char (*)[true || sizeof(typename T::missing)]) { return 0; }
int g() { return f<int>(nullptr); }
